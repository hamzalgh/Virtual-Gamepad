package com.hamzalgh.virtualgamepad.settings

@ConsistentCopyVisibility
data class Endpoint private constructor(val address: String, val port: Int) {
    companion object {
        fun parse(address: String, port: String): Endpoint? {
            val parts = address.trim().split('.')
            if (parts.size != 4 || parts.any {
                    it.isEmpty() || it.length > 3 || it.any { c -> c !in '0'..'9' } ||
                        (it.length > 1 && it[0] == '0') || (it.toIntOrNull() ?: 256) > 255
                }) return null
            val octets = parts.map { it.toInt() }
            val (a, b) = octets
            val local = a == 10 || (a == 172 && b in 16..31) ||
                (a == 192 && b == 168) || (a == 169 && b == 254) || a == 127
            val p = port.trim()
            val number = p.toIntOrNull() ?: return null
            if (!local || p.any { it !in '0'..'9' } || number !in 1..65535) return null
            return Endpoint(octets.joinToString("."), number)
        }
    }
}
