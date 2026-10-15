package com.hamzalgh.virtualgamepad.network

/** Bounded discovery state, independent of platform callbacks. Tickets prevent a
 * lost and rediscovered name from accepting the previous service's lookup result. */
class DiscoveryCatalog<T> {
    data class Lookup<T>(val key: String, val ticket: Long, val value: T)
    private var serial = 0L
    private val known = linkedMapOf<String, Long>()
    private val pending = linkedMapOf<String, Lookup<T>>()
    private val resolved = linkedMapOf<String, DiscoveredPc>()
    val pcs: List<DiscoveredPc> get() = resolved.values.sortedBy { it.name }
    val size: Int get() = known.size
    fun add(key: String, value: T): Boolean {
        if (known.size >= 32 || key in known) return false
        val ticket = ++serial
        known[key] = ticket; pending[key] = Lookup(key, ticket, value)
        return true
    }
    fun next(): Lookup<T>? = pending.entries.firstOrNull()?.let { pending.remove(it.key) }
    fun resolve(lookup: Lookup<T>, pc: DiscoveredPc?) {
        if (known[lookup.key] == lookup.ticket && pc != null) resolved[lookup.key] = pc
    }
    fun lost(key: String) { known.remove(key); pending.remove(key); resolved.remove(key) }
    fun clear() { known.clear(); pending.clear(); resolved.clear() }
}
