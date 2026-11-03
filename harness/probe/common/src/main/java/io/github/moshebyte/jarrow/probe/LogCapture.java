// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

/**
 * Collects every WARN, ERROR and FATAL log event from the moment the probe is installed.
 *
 * <p>The loader initialises mods after it has already logged its own start-up, so events from
 * before {@link #install()} are not seen here. The harness reads the full server log for those.
 * Each entry is {@code "[<logger>] <message>"}, plus {@code " (<exception>)"} when the event carries
 * one. Timestamps and thread names are left out so two runs of the same server compare equal.
 *
 * <p>This class uses no Log4j type. The hook into Log4j lives in {@link Log4jHook}, so a loader
 * that hides log4j-core from mods costs the dump its log section (reported in {@code "errors"})
 * instead of crashing the probe.
 */
public final class LogCapture {
    /** At most this many entries per list; the rest are counted, not kept. */
    static final int LIMIT = 5000;

    private static final Object LOCK = new Object();
    private static final List<String> WARNINGS = new ArrayList<>();
    private static final List<String> ERRORS = new ArrayList<>();
    private static int droppedWarnings;
    private static int droppedErrors;
    private static volatile String installError;
    private static volatile boolean installed;

    private LogCapture() {}

    /** Attaches the collector to the root logger of every live Log4j context. Safe to call twice. */
    public static void install() {
        if (installed) {
            return;
        }
        installed = true;
        try {
            int contexts = Log4jHook.attach();
            if (contexts == 0) {
                installError = "no Log4j logger context found";
            }
        } catch (RuntimeException | LinkageError e) {
            installError = e.toString();
        }
    }

    /** The {@code "log"} object of the dump: {@code {"errors": [...], "warnings": [...]}}. */
    public static Map<String, Object> snapshot(List<String> dumpErrors) {
        if (installError != null) {
            dumpErrors.add("log: capture unavailable: " + installError);
        }
        Map<String, Object> log = new TreeMap<>(Json.KEY_ORDER);
        synchronized (LOCK) {
            List<String> errors = new ArrayList<>(ERRORS);
            if (droppedErrors > 0) {
                errors.add("[jarrow_probe] " + droppedErrors + " more errors were logged and not recorded");
            }
            List<String> warnings = new ArrayList<>(WARNINGS);
            if (droppedWarnings > 0) {
                warnings.add("[jarrow_probe] " + droppedWarnings + " more warnings were logged and not recorded");
            }
            log.put("errors", errors);
            log.put("warnings", warnings);
        }
        return log;
    }

    /**
     * Records one event.
     *
     * @param error true for ERROR and FATAL, false for WARN
     */
    static void record(boolean error, String loggerName, String message, Throwable thrown) {
        StringBuilder line = new StringBuilder();
        line.append('[').append(loggerName == null || loggerName.isEmpty() ? "root" : loggerName).append("] ");
        line.append(message == null ? "" : message);
        if (thrown != null) {
            line.append(" (").append(thrown).append(')');
        }
        synchronized (LOCK) {
            if (error) {
                if (ERRORS.size() < LIMIT) {
                    ERRORS.add(line.toString());
                } else {
                    droppedErrors++;
                }
            } else if (WARNINGS.size() < LIMIT) {
                WARNINGS.add(line.toString());
            } else {
                droppedWarnings++;
            }
        }
    }
}
