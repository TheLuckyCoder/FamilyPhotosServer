package net.theluckycoder.familyphotos.utils

import org.springframework.web.filter.ShallowEtagHeaderFilter

class Sha512ShallowEtagHeaderFilter : ShallowEtagHeaderFilter() {
    protected fun generateETagHeaderValue(bytes: ByteArray?): String {
        val hash: HashCode = Hashing.sha512().hashBytes(bytes)
        return "\"" + hash + "\""
    }
}
