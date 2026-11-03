// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import org.apache.logging.log4j.Level;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;
import org.junit.jupiter.api.Test;

class LogCaptureTest {
    @Test
    void capturesWarningsAndErrorsAfterInstall() {
        LogCapture.install();
        Logger logger = LogManager.getLogger("jarrow.test.capture");
        logger.info("not captured");
        logger.warn("a warning {}", 1);
        logger.error("an error", new IllegalStateException("boom"));

        List<String> dumpErrors = new ArrayList<>();
        Map<String, Object> log = LogCapture.snapshot(dumpErrors);
        assertEquals(List.of(), dumpErrors);

        @SuppressWarnings("unchecked")
        List<String> warnings = (List<String>) log.get("warnings");
        @SuppressWarnings("unchecked")
        List<String> errors = (List<String>) log.get("errors");
        assertTrue(warnings.contains("[jarrow.test.capture] a warning 1"), warnings.toString());
        assertTrue(errors.contains("[jarrow.test.capture] an error (java.lang.IllegalStateException: boom)"),
                errors.toString());
        assertTrue(warnings.stream().noneMatch(w -> w.contains("not captured")));
        assertEquals(List.of("errors", "warnings"), new ArrayList<>(log.keySet()));
    }

    @Test
    void fatalGoesToErrors() {
        LogCapture.install();
        LogManager.getLogger("jarrow.test.fatal").log(Level.FATAL, "fatal line");
        LogCapture.record(true, "", "unnamed logger", null);
        @SuppressWarnings("unchecked")
        List<String> errors = (List<String>) LogCapture.snapshot(new ArrayList<>()).get("errors");
        assertTrue(errors.contains("[jarrow.test.fatal] fatal line"), errors.toString());
        assertTrue(errors.contains("[root] unnamed logger"), errors.toString());
    }
}
