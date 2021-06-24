package net.theluckycoder.familyphotos.service

import net.theluckycoder.familyphotos.repository.UserRepository
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.security.core.GrantedAuthority
import org.springframework.security.core.authority.SimpleGrantedAuthority
import org.springframework.security.core.userdetails.UserDetails
import org.springframework.security.core.userdetails.UserDetailsService
import org.springframework.security.core.userdetails.UsernameNotFoundException
import org.springframework.stereotype.Service

@Service
class MyUserDetailsService @Autowired constructor(
    private val userRepository: UserRepository
) : UserDetailsService {

    @Throws(UsernameNotFoundException::class)
    override fun loadUserByUsername(username: String): UserDetails {
        val user = userRepository.findByUserName(username).orElseThrow {
            UsernameNotFoundException("Not found: $username")
        }

        val roles = user.roles.splitToSequence(',').map { SimpleGrantedAuthority(it) }.toList()

        println(roles)

        return MyUserDetails(
            user.userName,
            user.password,
            user.active,
            roles
        )
    }

    class MyUserDetails(
        private val username: String,
        private val password: String,
        private val active: Boolean,
        private val authorities: List<GrantedAuthority>
    ) : UserDetails {

        override fun getAuthorities() = authorities

        override fun getPassword(): String = password

        override fun getUsername(): String = username

        override fun isAccountNonExpired(): Boolean = true

        override fun isAccountNonLocked(): Boolean = true

        override fun isCredentialsNonExpired(): Boolean = true

        override fun isEnabled(): Boolean = active
    }
}
