package net.theluckycoder.familyphotos.configs;

import org.apache.coyote.http11.Http11AprProtocol;
import org.springframework.beans.BeansException;
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty;
import org.springframework.boot.autoconfigure.web.ServerProperties;
import org.springframework.boot.web.embedded.tomcat.TomcatServletWebServerFactory;
import org.springframework.boot.web.server.Ssl;
import org.springframework.context.ApplicationContext;
import org.springframework.context.ApplicationContextAware;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

import static java.util.Arrays.asList;
import static java.util.Collections.singletonList;
import static org.apache.logging.log4j.util.Strings.join;

@Configuration
@ConditionalOnProperty(value = {"server.ssl.enabled", "server.ssl.aprEnabled"})
public class TcNativeConfig implements ApplicationContextAware {
    private String certificateFile, certificateKeyFile;

    @Override
    public void setApplicationContext(ApplicationContext applicationContext) throws BeansException {
        // It's necessary to read these properties directly from the app context, as the config is loaded
        // too early in the app lifecycle for @Value handlers to work properly
        certificateFile = applicationContext.getEnvironment().getProperty("server.ssl.certificateFile");
        certificateKeyFile = applicationContext.getEnvironment().getProperty("server.ssl.certificateKeyFile");
    }

    @Bean
    public TomcatServletWebServerFactory tomcatServletWebServerFactory(ServerProperties serverProperties) {
        TomcatServletWebServerFactory factory = new TomcatServletWebServerFactory() {
            @Override
            public Ssl getSsl() {
                // Need to do this in order to stop the default SSL customizer from running and imposing its
                // requirement on us using the JSSE Protocol for SSL
                return null;
            }
        };

        // This is the protocol that will enable APR
        factory.setProtocol("org.apache.coyote.http11.Http11AprProtocol");
        factory.setTomcatConnectorCustomizers(singletonList(connector -> {
            connector.setScheme("https");
            connector.setSecure(true);

            Ssl ssl = serverProperties.getSsl();
            Http11AprProtocol protocol = (Http11AprProtocol) connector.getProtocolHandler();
            protocol.setSSLEnabled(true);
            protocol.setSSLCertificateKeyFile(certificateKeyFile);
            protocol.setSSLCertificateFile(certificateFile);

            String[] protocols = ssl.getEnabledProtocols();
            if (protocols != null && protocols.length > 0) {
                protocol.setSslEnabledProtocols(join(asList(protocols), ','));
            }

            String[] ciphers = ssl.getCiphers();
            if (ciphers != null && ciphers.length > 0) {
//                    LOG.info("Using cipher list {}", ciphers[0]);
                protocol.setCiphers(ciphers[0]);
            }
        }));

        return factory;
    }
}
