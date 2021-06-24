package net.theluckycoder.familyphotos.model

data class SimpleUser(
    val id: Long,
    val userName: String,
    val displayName: String,
)

fun User.toSimpleUser() = SimpleUser(id, userName, displayName)
