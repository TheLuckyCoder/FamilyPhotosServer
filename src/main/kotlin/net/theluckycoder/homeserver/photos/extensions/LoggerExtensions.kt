package net.theluckycoder.homeserver.photos.extensions

import org.slf4j.Logger
import org.slf4j.LoggerFactory

object LoggerExtensions {
    inline fun <reified T> getLogger(): Logger = LoggerFactory.getLogger(T::class.java)
}

