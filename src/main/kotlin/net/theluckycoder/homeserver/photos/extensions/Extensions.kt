package net.theluckycoder.homeserver.photos.extensions

import java.io.File
import javax.servlet.ServletContext

fun ServletContext.getMimeTypeAll(file: File): String {
    return try {
        requireNotNull(getMimeType(file.absolutePath)) { "MimeType can not be null" }
    } catch (e: IllegalArgumentException) { // Catches InvalidMediaTypeException as well
        when (file.extension) {
            "heif", "heic" -> "image/heif"
            else -> throw e
        }
    }
}
