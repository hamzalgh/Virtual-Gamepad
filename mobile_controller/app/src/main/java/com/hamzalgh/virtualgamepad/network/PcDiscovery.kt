package com.hamzalgh.virtualgamepad.network

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.LinkProperties
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.os.Handler
import android.os.Looper
import com.hamzalgh.virtualgamepad.settings.Endpoint
import java.net.Inet4Address

data class DiscoveredPc(val key: String, val name: String, val endpoint: Endpoint)
data class DiscoveryState(val searching: Boolean = false, val pcs: List<DiscoveredPc> = emptyList(),
    val detail: String = "Find PCs, or enter an address manually.")

fun discoveredPc(key: String, name: String, address: String, port: Int, attributes: Map<String, String>): DiscoveredPc? {
    if (attributes["v"] != "1" || attributes["capacity"] != "1" || attributes["pairing"] != "required") return null
    val endpoint = Endpoint.parse(address, port.toString()) ?: return null
    if (address.startsWith("127.")) return null
    val label = name.filter { !it.isISOControl() }.take(64).ifBlank { "Virtual Gamepad PC" }
    return DiscoveredPc(key, label, endpoint)
}

/** Main-thread ownership; at most 32 services and one unresolved platform request.
 * Old Android cannot cancel resolveService: keep its slot occupied until completion,
 * and ignore its result after cancellation. Never accumulate abandoned resolvers.
 */
class PcDiscovery(context: Context, private val publish: (DiscoveryState) -> Unit) {
    private val nsd = context.getSystemService(NsdManager::class.java)
    private val connectivity = context.getSystemService(ConnectivityManager::class.java)
    private val main = Handler(Looper.getMainLooper())
    private var generation = 0
    private var listener: NsdManager.DiscoveryListener? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private val catalog = DiscoveryCatalog<NsdServiceInfo>()
    private var resolving: String? = null
    private var legacyBusy = false
    private var resolutionId = 0
    private var cancelResolution: (() -> Unit)? = null
    private var timeout: Runnable? = null
    private var state = DiscoveryState()
    private fun show(detail: String = state.detail) {
        state = DiscoveryState(listener != null, catalog.pcs, detail)
        publish(state)
    }
    fun stop(detail: String = "Discovery stopped. Find PCs again, or enter an address manually.") {
        generation++
        listener?.let { runCatching { nsd.stopServiceDiscovery(it) } }; listener = null
        networkCallback?.let { runCatching { connectivity.unregisterNetworkCallback(it) } }; networkCallback = null
        cancelResolution?.invoke(); cancelResolution = null
        timeout?.let(main::removeCallbacks); timeout = null
        resolving = null; catalog.clear(); show(detail)
    }
    fun start() {
        stop()
        if (legacyBusy) { show("Android is finishing a previous lookup. Retry shortly, or use manual IP."); return }
        val token = generation
        val initialNetwork = connectivity.activeNetwork
        if (initialNetwork == null) { show("Connect to your private Wi-Fi, then find PCs again."); return }
        val initialLinks = connectivity.getLinkProperties(initialNetwork)?.let { it.interfaceName to it.linkAddresses.toSet() }
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onLinkPropertiesChanged(network: Network, links: LinkProperties) { main.post {
                if (token == generation && initialLinks != null && initialLinks != (links.interfaceName to links.linkAddresses.toSet()))
                    stop("Network address changed. Find PCs again.")
            } }
            override fun onLost(network: Network) { main.post { if (token == generation) stop("Network changed. Find PCs again on your private Wi-Fi.") } }
            override fun onAvailable(network: Network) { main.post { if (token == generation && network != initialNetwork) stop("Network changed. Find PCs again.") } }
        }
        val discovery = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(type: String) { main.post { if (token == generation) show("Searching your local network. Manual IP is always available.") } }
            override fun onDiscoveryStopped(type: String) { main.post { if (token == generation) stop("Discovery stopped. Find PCs again, or use manual IP.") } }
            override fun onStartDiscoveryFailed(type: String, error: Int) { main.post { if (token == generation) stop("Discovery unavailable. Check permission and Wi-Fi, or use manual IP.") } }
            override fun onStopDiscoveryFailed(type: String, error: Int) {}
            override fun onServiceFound(info: NsdServiceInfo) { main.post {
                if (token != generation || info.serviceType.trim('.') != "_virtual-gamepad._udp") return@post
                val key = info.serviceName
                if (catalog.add(key, info)) next(token)
            } }
            override fun onServiceLost(info: NsdServiceInfo) { main.post {
                if (token != generation) return@post
                catalog.lost(info.serviceName)
                show()
            } }
        }
        listener = discovery
        try {
            connectivity.registerDefaultNetworkCallback(callback); networkCallback = callback
            nsd.discoverServices("_virtual-gamepad._udp.", NsdManager.PROTOCOL_DNS_SD, discovery)
            show("Searching your local network…")
        } catch (_: RuntimeException) { stop("Discovery unavailable. Check local network permission, or use manual IP.") }
    }
    private fun next(token: Int) {
        if (token != generation || resolving != null || legacyBusy) return
        val lookup = catalog.next() ?: return
        val key = lookup.key; val info = lookup.value
        resolving = key
        val requestId = ++resolutionId
        fun completed(resolved: NsdServiceInfo?) {
            if (token != generation || resolving != key || requestId != resolutionId) return
            timeout?.let(main::removeCallbacks); timeout = null
            cancelResolution?.invoke(); cancelResolution = null; resolving = null
            if (resolved != null) {
                @Suppress("DEPRECATION")
                val addresses = if (Build.VERSION.SDK_INT >= 34) resolved.hostAddresses else listOfNotNull(resolved.host)
                val attrs = resolved.attributes.mapValues { (_, value) -> value?.toString(Charsets.UTF_8).orEmpty() }
                addresses.filterIsInstance<Inet4Address>().firstNotNullOfOrNull {
                    discoveredPc(key, resolved.serviceName, it.hostAddress.orEmpty(), resolved.port, attrs)
                }?.let { catalog.resolve(lookup, it) }
            }
            show(if (catalog.pcs.isEmpty()) "No compatible PC resolved yet. Manual IP is available." else "Select a PC to fill its address, then Connect.")
            next(token)
        }
        timeout = Runnable { if (token == generation && resolving == key) {
            if (Build.VERSION.SDK_INT < 34) stop("PC lookup timed out. Use manual IP or retry after Android finishes the lookup.")
            else completed(null)
        } }.also { main.postDelayed(it, 5000) }
        try {
            if (Build.VERSION.SDK_INT >= 34) {
                val callback = object : NsdManager.ServiceInfoCallback {
                    override fun onServiceInfoCallbackRegistrationFailed(errorCode: Int) { main.post { completed(null) } }
                    override fun onServiceUpdated(serviceInfo: NsdServiceInfo) { main.post { completed(serviceInfo) } }
                    override fun onServiceLost() { main.post { completed(null) } }
                    override fun onServiceInfoCallbackUnregistered() {}
                }
                cancelResolution = { runCatching { nsd.unregisterServiceInfoCallback(callback) }; Unit }
                nsd.registerServiceInfoCallback(info, { command -> main.post(command) }, callback)
            } else {
                legacyBusy = true
                @Suppress("DEPRECATION")
                nsd.resolveService(info, object : NsdManager.ResolveListener {
                    override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) { main.post { legacyBusy = false; completed(null) } }
                    override fun onServiceResolved(serviceInfo: NsdServiceInfo) { main.post { legacyBusy = false; completed(serviceInfo) } }
                })
            }
        } catch (_: RuntimeException) { legacyBusy = false; completed(null) }
    }
}
