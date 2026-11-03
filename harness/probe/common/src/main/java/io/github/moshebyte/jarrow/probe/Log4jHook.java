// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import java.util.LinkedHashSet;
import java.util.Set;
import org.apache.logging.log4j.Level;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.core.LogEvent;
import org.apache.logging.log4j.core.LoggerContext;
import org.apache.logging.log4j.core.appender.AbstractAppender;
import org.apache.logging.log4j.core.config.Configuration;
import org.apache.logging.log4j.core.config.Property;
import org.apache.logging.log4j.core.impl.Log4jContextFactory;
import org.apache.logging.log4j.spi.LoggerContextFactory;

/**
 * The only class that touches log4j-core. {@link LogCapture} loads it inside a try block, so a
 * missing or hidden log4j-core is a recorded dump error, not a crash.
 */
final class Log4jHook {
    private static final String NAME = "JarrowProbeCapture";

    private Log4jHook() {}

    /** Adds the capture appender to the root logger of every live context; returns how many. */
    static int attach() {
        Appender appender = new Appender();
        appender.start();
        Set<LoggerContext> contexts = contexts();
        for (LoggerContext context : contexts) {
            Configuration config = context.getConfiguration();
            config.addAppender(appender);
            config.getRootLogger().addAppender(appender, Level.WARN, null);
            context.updateLoggers();
        }
        return contexts.size();
    }

    /**
     * Every logger context Log4j knows. Loaders give mods their own class loader, so the context
     * "for this class" may not be the one the server logs through; attaching to all of them
     * covers that.
     */
    private static Set<LoggerContext> contexts() {
        Set<LoggerContext> contexts = new LinkedHashSet<>();
        LoggerContextFactory factory = LogManager.getFactory();
        if (factory instanceof Log4jContextFactory log4j) {
            contexts.addAll(log4j.getSelector().getLoggerContexts());
        }
        if (LogManager.getContext(false) instanceof LoggerContext current) {
            contexts.add(current);
        }
        return contexts;
    }

    private static final class Appender extends AbstractAppender {
        Appender() {
            super(NAME, null, null, true, Property.EMPTY_ARRAY);
        }

        @Override
        public void append(LogEvent event) {
            Level level = event.getLevel();
            if (level == null || !level.isMoreSpecificThan(Level.WARN)) {
                return;
            }
            String message = event.getMessage() == null ? "" : event.getMessage().getFormattedMessage();
            LogCapture.record(level.isMoreSpecificThan(Level.ERROR), event.getLoggerName(), message, event.getThrown());
        }
    }
}
