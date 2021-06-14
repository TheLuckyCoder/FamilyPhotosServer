package net.theluckycoder.homeserver.photos.repository

import net.theluckycoder.homeserver.photos.exceptions.UserNotFoundException
import net.theluckycoder.homeserver.photos.model.User
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository
import org.springframework.data.repository.query.Param
import org.springframework.stereotype.Repository
import java.util.*

@Repository
interface UserRepository : CrudRepository<User, Long> {

    @Query("FROM User user WHERE user.userName=:name")
    fun findByUserName(@Param("name") userName: String): Optional<User>
}

fun UserRepository.findByIdOrThrow(userId: Long) =
    findById(userId).orElseThrow { UserNotFoundException("There is no User with id $userId") }
