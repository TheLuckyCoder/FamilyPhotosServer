package net.theluckycoder.familyphotos.configs

import org.springframework.beans.factory.annotation.Autowired
import org.springframework.context.annotation.Bean
import org.springframework.security.config.annotation.authentication.builders.AuthenticationManagerBuilder
import org.springframework.security.config.annotation.web.builders.HttpSecurity
import org.springframework.security.config.annotation.web.builders.WebSecurity
import org.springframework.security.config.annotation.web.configuration.EnableWebSecurity
import org.springframework.security.config.annotation.web.configuration.WebSecurityConfigurerAdapter
import org.springframework.security.core.userdetails.UserDetailsService
import org.springframework.security.crypto.bcrypt.BCryptPasswordEncoder
import org.springframework.security.crypto.password.PasswordEncoder

@EnableWebSecurity
class SecurityConfiguration @Autowired constructor(
    private val userDetailsService: UserDetailsService
) : WebSecurityConfigurerAdapter() {

    override fun configure(auth: AuthenticationManagerBuilder) {
        auth.userDetailsService(userDetailsService)
    }

    override fun configure(web: WebSecurity) {
        web.ignoring()
            .antMatchers("/db/**")
    }

    override fun configure(http: HttpSecurity) {
        http.antMatcher("/**")
            .authorizeRequests()
            .antMatchers("/", "db/**").permitAll()
            .anyRequest().authenticated()
            .and().formLogin()
            .and().httpBasic()
    }

    @Bean
    fun getPasswordEncoder(): PasswordEncoder =
        BCryptPasswordEncoder(BCryptPasswordEncoder.BCryptVersion.`$2B`, 8)

    object Role {
//        const val ADMIN = "ADMIN"
        const val USER = "USER"
        const val PUBLIC = "PUBLIC"
    }
}