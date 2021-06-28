package net.theluckycoder.familyphotos.configs

import org.springframework.scheduling.annotation.EnableAsync
import org.springframework.scheduling.annotation.EnableScheduling
import org.springframework.scheduling.annotation.AsyncConfigurer
import net.theluckycoder.familyphotos.configs.AsyncConfiguration
import org.slf4j.LoggerFactory
import org.springframework.core.task.AsyncTaskExecutor
import org.springframework.scheduling.concurrent.ThreadPoolTaskExecutor
import org.springframework.aop.interceptor.AsyncUncaughtExceptionHandler
import org.springframework.aop.interceptor.SimpleAsyncUncaughtExceptionHandler
import org.springframework.beans.factory.annotation.Qualifier
import org.springframework.context.annotation.Bean
import org.springframework.context.annotation.Configuration
import org.springframework.web.context.request.async.CallableProcessingInterceptor
import org.springframework.web.servlet.config.annotation.WebMvcConfigurer
import org.springframework.web.servlet.config.annotation.AsyncSupportConfigurer
import org.springframework.web.context.request.async.TimeoutCallableProcessingInterceptor
import kotlin.Throws
import org.springframework.web.context.request.NativeWebRequest
import java.lang.Exception
import java.util.concurrent.Callable

@Configuration
@EnableAsync
@EnableScheduling
class AsyncConfiguration : AsyncConfigurer {

    private val log = LoggerFactory.getLogger(AsyncConfiguration::class.java)

    @Bean(name = ["taskExecutor"])
    override fun getAsyncExecutor(): AsyncTaskExecutor {
        log.debug("Creating Async Task Executor")
       return ThreadPoolTaskExecutor().apply {
            corePoolSize = 4
            maxPoolSize = 8
            setQueueCapacity(25)
        }
    }

    override fun getAsyncUncaughtExceptionHandler(): AsyncUncaughtExceptionHandler? =
        SimpleAsyncUncaughtExceptionHandler()

    /** Configure async support for Spring MVC.  */
    @Bean
    fun webMvcConfigurerConfigurer(
        @Qualifier("taskExecutor") taskExecutor: AsyncTaskExecutor,
        callableProcessingInterceptor: CallableProcessingInterceptor
    ) = object : WebMvcConfigurer {
        override fun configureAsyncSupport(configurer: AsyncSupportConfigurer) {
            configurer.setDefaultTimeout(360000).setTaskExecutor(taskExecutor)
            configurer.registerCallableInterceptors(callableProcessingInterceptor)
            super.configureAsyncSupport(configurer)
        }
    }

    @Bean
    fun callableProcessingInterceptor(): CallableProcessingInterceptor {
        return object : TimeoutCallableProcessingInterceptor() {
            @Throws(Exception::class)
            override fun <T> handleTimeout(request: NativeWebRequest, task: Callable<T>): Any {
                log.error("timeout!")
                return super.handleTimeout(request, task)
            }
        }
    }
}
