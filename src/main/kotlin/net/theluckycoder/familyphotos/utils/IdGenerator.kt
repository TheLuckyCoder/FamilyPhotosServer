package net.theluckycoder.familyphotos.utils

import java.security.SecureRandom

object IdGenerator {

    private val random by lazy {
        SecureRandom.getInstance("SHA1PRNG", "SUN").apply {
            setSeed(SecureRandom.getSeed(256))
        }
    }

    @Synchronized
    fun get() = random.nextLong()
}
