package com.hamzalgh.virtualgamepad.network

import java.util.concurrent.atomic.AtomicReference

data class CadenceStats(
    val sent: Long = 0,
    val skipped: Long = 0,
    val intervals: Int = 0,
    val minMs: Long = 0,
    val maxMs: Long = 0,
    val meanMs: Double = 0.0,
)

/** Single sender writer, atomic immutable snapshot for the receiver. No per tick publication. */
class CadenceMeter {
    private val published = AtomicReference(CadenceStats())
    private var previous: Long? = null
    private var windowStart = 0L
    private var sent = 0L
    private var skipped = 0L
    private var intervals = 0
    private var min = Long.MAX_VALUE
    private var max = 0L
    private var mean = 0.0

    fun snapshot(): CadenceStats = published.get()

    fun skip(count: Long) { skipped += count.coerceIn(0, Long.MAX_VALUE - skipped) }

    fun record(now: Long) {
        if (sent < Long.MAX_VALUE) sent++
        val last = previous
        if (last == null) windowStart = now
        else {
            val interval = (now - last).coerceAtLeast(0)
            intervals++
            min = minOf(min, interval)
            max = maxOf(max, interval)
            mean += (interval - mean) / intervals
        }
        previous = now
        if (now - windowStart >= 500) {
            published.set(CadenceStats(sent, skipped, intervals, if (intervals == 0) 0 else min, max, mean))
            windowStart = now
            intervals = 0
            min = Long.MAX_VALUE
            max = 0
            mean = 0.0
        }
    }
}
