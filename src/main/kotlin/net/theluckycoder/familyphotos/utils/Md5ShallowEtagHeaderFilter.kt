package net.theluckycoder.familyphotos.utils

import org.springframework.util.DigestUtils
import org.springframework.web.filter.ShallowEtagHeaderFilter
import java.io.InputStream

class Md5ShallowEtagHeaderFilter : ShallowEtagHeaderFilter() {
    fun generateETagHeaderValue(inputStream: InputStream): String {
        val builder = StringBuilder(37)

        builder.append('"')
        DigestUtils.appendMd5DigestAsHex(inputStream, builder)
        builder.append('"')

        return builder.toString()
    }
}
