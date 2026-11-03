// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class ProbeDumpTest {
    @Test
    void fixedFacts() {
        assertEquals("jarrow-dump.json", ProbeDump.FILE_NAME);
        assertEquals(1, ProbeDump.SCHEMA);
        assertArrayEquals(new int[][] {{0, 0}, {8, 8}, {-8, 8}, {8, -8}}, ProbeDump.CHUNKS);
    }

    @Test
    void writeReplacesAnOldDumpAndLeavesNoTempFile(@TempDir Path dir) throws Exception {
        Path out = dir.resolve(ProbeDump.FILE_NAME);
        Files.writeString(out, "stale");
        Map<String, Object> dump = new TreeMap<>(Json.KEY_ORDER);
        dump.put("schema", 1);
        dump.put("errors", List.of());
        ProbeDump.write(out, dump);
        assertEquals("{\n  \"errors\": [],\n  \"schema\": 1\n}\n", Files.readString(out, StandardCharsets.UTF_8));
        assertFalse(Files.exists(dir.resolve(ProbeDump.FILE_NAME + ".tmp")));
    }

    @Test
    void modsAreSortedById() {
        LoaderInfo info = new LoaderInfo("fabric", "0.20.1", "1.21.1", List.of(
                new LoaderInfo.Mod("minecraft", "1.21.1"),
                new LoaderInfo.Mod("fabricloader", "0.20.1"),
                new LoaderInfo.Mod("jarrow_probe", "0.1.0")));
        assertEquals(List.of("fabricloader", "jarrow_probe", "minecraft"),
                info.sortedMods().stream().map(LoaderInfo.Mod::id).toList());
    }
}
