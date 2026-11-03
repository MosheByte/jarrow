// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.JsonArray;
import com.google.gson.JsonNull;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class JsonTest {
    @Test
    void sortsObjectKeysWhateverTheMapOrder() {
        Map<String, Object> map = new LinkedHashMap<>();
        map.put("zeta", 1);
        map.put("alpha", 2);
        map.put("Beta", 3);
        assertEquals("{\n  \"Beta\": 3,\n  \"alpha\": 2,\n  \"zeta\": 1\n}\n", Json.toJson(map));
    }

    @Test
    void keyOrderIsCodePointOrderNotUtf16Order() {
        // U+FF5E sorts before U+1F600 by code point (and by UTF-8 bytes), but after its
        // surrogate pair by UTF-16 code unit, which is what String.compareTo uses.
        String fullwidthTilde = "～";
        String emoji = new String(Character.toChars(0x1F600));
        assertTrue(fullwidthTilde.compareTo(emoji) > 0);
        assertTrue(Json.compareCodePoints(fullwidthTilde, emoji) < 0);
        List<String> keys = new ArrayList<>(List.of(emoji, fullwidthTilde, "a"));
        keys.sort(Json.KEY_ORDER);
        assertEquals(List.of("a", fullwidthTilde, emoji), keys);
    }

    @Test
    void prefixSortsFirst() {
        assertTrue(Json.compareCodePoints("minecraft:stone", "minecraft:stone_bricks") < 0);
        assertEquals(0, Json.compareCodePoints("x", "x"));
    }

    @Test
    void escapesStrings() {
        assertEquals("\"a\\\"b\\\\c\\n\\t\\u0001\"\n", Json.toJson("a\"b\\c\n\t\u0001"));
        assertEquals("\"café ☃\"\n", Json.toJson("café ☃"));
    }

    @Test
    void replacesLoneSurrogates() {
        assertEquals("\"x�y\"\n", Json.toJson("x\uD800y"));
        String pair = new String(Character.toChars(0x1F600));
        assertEquals("\"" + pair + "\"\n", Json.toJson(pair));
    }

    @Test
    void writesNumbers() {
        assertEquals("[\n  1,\n  -2,\n  3,\n  4,\n  0.5,\n  0.1,\n  12345678901234567890\n]\n",
                Json.toJson(List.of(1, -2L, (short) 3, (byte) 4, 0.5d, 0.1f,
                        new BigInteger("12345678901234567890"))));
    }

    @Test
    void nonFiniteNumbersBecomeStrings() {
        assertEquals("[\n  \"NaN\",\n  \"Infinity\"\n]\n",
                Json.toJson(List.of(Double.NaN, Float.POSITIVE_INFINITY)));
    }

    @Test
    void emptyContainersAndNull() {
        Map<String, Object> map = new HashMap<>();
        map.put("a", List.of());
        map.put("b", Map.of());
        map.put("c", null);
        map.put("d", true);
        assertEquals("{\n  \"a\": [],\n  \"b\": {},\n  \"c\": null,\n  \"d\": true\n}\n", Json.toJson(map));
    }

    @Test
    void writesGsonTreesWithSortedKeys() {
        JsonObject obj = new JsonObject();
        obj.addProperty("type", "minecraft:crafting_shaped");
        obj.addProperty("count", 4);
        obj.addProperty("chance", 0.25f);
        obj.add("missing", JsonNull.INSTANCE);
        JsonArray pattern = new JsonArray();
        pattern.add("##");
        pattern.add(true);
        obj.add("pattern", pattern);
        assertEquals(
                "{\n  \"chance\": 0.25,\n  \"count\": 4,\n  \"missing\": null,\n"
                        + "  \"pattern\": [\n    \"##\",\n    true\n  ],\n"
                        + "  \"type\": \"minecraft:crafting_shaped\"\n}\n",
                Json.toJson(obj));
    }

    @Test
    void parsedGsonNumbersKeepTheirText() {
        assertEquals("[\n  1.50,\n  7\n]\n", Json.toJson(JsonParser.parseString("[1.50, 7]")));
    }

    @Test
    void outputParsesBackToTheSameTree() {
        Map<String, Object> map = new HashMap<>();
        map.put("registries", Map.of("minecraft:block", List.of("minecraft:air", "minecraft:stone")));
        map.put("schema", 1);
        map.put("seed", Long.MIN_VALUE);
        String text = Json.toJson(map);
        JsonObject parsed = JsonParser.parseString(text).getAsJsonObject();
        assertEquals(1, parsed.get("schema").getAsInt());
        assertEquals(Long.MIN_VALUE, parsed.get("seed").getAsLong());
        assertEquals("minecraft:stone",
                parsed.getAsJsonObject("registries").getAsJsonArray("minecraft:block").get(1).getAsString());
    }

    @Test
    void rejectsUnknownValuesAndNonStringKeys() {
        assertThrows(IllegalArgumentException.class, () -> Json.toJson(new Object()));
        assertThrows(IllegalArgumentException.class, () -> Json.toJson(Map.of(1, "x")));
    }

    @Test
    void numberTextRejectsWhatJsonCannotHold() {
        assertEquals(null, Json.numberText(Double.NEGATIVE_INFINITY));
        assertEquals("1.0E10", Json.numberText(1.0e10));
    }
}
