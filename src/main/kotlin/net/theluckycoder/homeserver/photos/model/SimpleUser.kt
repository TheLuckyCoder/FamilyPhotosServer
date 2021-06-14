package net.theluckycoder.homeserver.photos.model

data class SimpleUser(
    val id: Long,
    val userName: String,
    val displayName: String,
)

fun User.toSimpleUser() = SimpleUser(id, userName, displayName)
