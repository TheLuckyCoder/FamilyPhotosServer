package net.theluckycoder.homeserver.photos.controller

import net.theluckycoder.homeserver.photos.model.SimpleUser
import net.theluckycoder.homeserver.photos.model.toSimpleUser
import net.theluckycoder.homeserver.photos.repository.UserRepository
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.RestController

@RestController
class UserController @Autowired constructor(
    private val userRepository: UserRepository,
) {

    @GetMapping("/users")
    fun getUsers(): Iterable<SimpleUser> = userRepository.findAll().map { it.toSimpleUser() }

    @GetMapping("/user/{userName}")
    fun getUser(@PathVariable userName: String): SimpleUser? {
        val user = userRepository.findByUserName(userName).orElseGet { null }
        return user?.toSimpleUser()
    }
}
