package net.theluckycoder.familyphotos.extensions

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
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

fun <T, R> Sequence<T>.asyncForEach(
    coroutineScope: CoroutineScope,
    coroutineDispatcher: CoroutineDispatcher = Dispatchers.Default,
    action: suspend CoroutineScope.(T) -> R
): Sequence<Deferred<R>> =
    AsyncForEach(this, coroutineScope, coroutineDispatcher, action)

internal class AsyncForEach<T, R>(
    private val sequence: Sequence<T>,
    private val coroutineScope: CoroutineScope,
    private val coroutineDispatcher: CoroutineDispatcher,
    private val action: suspend CoroutineScope.(T) -> R
) : Sequence<Deferred<R>> {

    override fun iterator(): Iterator<Deferred<R>> = object : Iterator<Deferred<R>> {
        val iterator = sequence.iterator()

        @Suppress("DeferredIsResult")
        override fun next(): Deferred<R> {
            val next = iterator.next()
            return coroutineScope.async(coroutineDispatcher) { action(next) }
        }

        override fun hasNext(): Boolean = iterator.hasNext()
    }
}
