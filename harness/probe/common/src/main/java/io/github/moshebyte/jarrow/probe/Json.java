// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonPrimitive;
import java.io.IOException;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;

/**
 * A small canonical JSON writer.
 *
 * <p>Object keys are always written in Unicode code point order, which is the byte order of their
 * UTF-8 encoding, so a Rust {@code BTreeMap<String, _>} reads them back in the same order. Output
 * is pretty-printed with two-space indentation and ends with a newline. Lone UTF-16 surrogates are
 * replaced with U+FFFD so every string is valid Unicode.
 *
 * <p>Accepted values: {@code null}, {@link Boolean}, {@link CharSequence}, {@link Number}, {@link
 * Map} with string keys, {@link Collection}, and Gson's {@link JsonElement} tree (what Minecraft's
 * codecs produce with {@code JsonOps}). Anything else is a programming error.
 */
public final class Json {
    /** Unicode code point order: the same order as comparing UTF-8 bytes. */
    public static final Comparator<String> KEY_ORDER = Json::compareCodePoints;

    private static final Pattern JSON_NUMBER =
            Pattern.compile("-?(?:0|[1-9][0-9]*)(?:\\.[0-9]+)?(?:[eE][+-]?[0-9]+)?");

    private Json() {}

    /** Writes {@code value} as pretty-printed canonical JSON followed by a newline. */
    public static void write(Object value, Appendable out) throws IOException {
        new Writer(out).value(value, 0);
        out.append('\n');
    }

    /** Returns {@code value} as pretty-printed canonical JSON followed by a newline. */
    public static String toJson(Object value) {
        StringBuilder sb = new StringBuilder();
        try {
            write(value, sb);
        } catch (IOException e) {
            throw new IllegalStateException("StringBuilder never throws", e);
        }
        return sb.toString();
    }

    /** Compares two strings by Unicode code point, not by UTF-16 code unit. */
    public static int compareCodePoints(String a, String b) {
        int i = 0;
        int j = 0;
        while (i < a.length() && j < b.length()) {
            int ca = a.codePointAt(i);
            int cb = b.codePointAt(j);
            if (ca != cb) {
                return Integer.compare(ca, cb);
            }
            i += Character.charCount(ca);
            j += Character.charCount(cb);
        }
        return Integer.compare(a.length() - i, b.length() - j);
    }

    /** The JSON text of a number, or {@code null} when it has none (NaN, infinities). */
    static String numberText(Number n) {
        if (n instanceof Integer || n instanceof Long || n instanceof Short || n instanceof Byte
                || n instanceof BigInteger) {
            return n.toString();
        }
        if (n instanceof Double d) {
            return d.isNaN() || d.isInfinite() ? null : d.toString();
        }
        if (n instanceof Float f) {
            return f.isNaN() || f.isInfinite() ? null : f.toString();
        }
        if (n instanceof BigDecimal bd) {
            return bd.toString();
        }
        String text = n.toString();
        return JSON_NUMBER.matcher(text).matches() ? text : null;
    }

    private static final class Writer {
        private final Appendable out;

        Writer(Appendable out) {
            this.out = out;
        }

        void value(Object value, int depth) throws IOException {
            if (value == null) {
                out.append("null");
            } else if (value instanceof Boolean b) {
                out.append(b ? "true" : "false");
            } else if (value instanceof CharSequence s) {
                string(s.toString());
            } else if (value instanceof Number n) {
                number(n);
            } else if (value instanceof Map<?, ?> map) {
                object(map, depth);
            } else if (value instanceof Collection<?> list) {
                array(list, depth);
            } else if (value instanceof JsonElement element) {
                gson(element, depth);
            } else {
                throw new IllegalArgumentException(
                        "cannot write a " + value.getClass().getName() + " as JSON");
            }
        }

        private void gson(JsonElement element, int depth) throws IOException {
            if (element.isJsonNull()) {
                out.append("null");
            } else if (element instanceof JsonPrimitive p) {
                if (p.isBoolean()) {
                    out.append(p.getAsBoolean() ? "true" : "false");
                } else if (p.isNumber()) {
                    number(p.getAsNumber());
                } else {
                    string(p.getAsString());
                }
            } else if (element instanceof JsonObject obj) {
                object(obj.asMap(), depth);
            } else if (element instanceof JsonArray arr) {
                array(arr.asList(), depth);
            } else {
                throw new IllegalArgumentException("unknown Gson element " + element.getClass().getName());
            }
        }

        private void number(Number n) throws IOException {
            String text = numberText(n);
            if (text == null) {
                // JSON has no NaN or infinity; keep the value visible instead of dropping it.
                string(n.toString());
            } else {
                out.append(text);
            }
        }

        private void object(Map<?, ?> map, int depth) throws IOException {
            if (map.isEmpty()) {
                out.append("{}");
                return;
            }
            List<String> keys = new ArrayList<>(map.size());
            for (Object key : map.keySet()) {
                if (!(key instanceof String s)) {
                    throw new IllegalArgumentException("JSON object keys must be strings, got " + key);
                }
                keys.add(s);
            }
            keys.sort(KEY_ORDER);
            out.append('{');
            boolean first = true;
            for (String key : keys) {
                out.append(first ? "\n" : ",\n");
                first = false;
                indent(depth + 1);
                string(key);
                out.append(": ");
                value(map.get(key), depth + 1);
            }
            out.append('\n');
            indent(depth);
            out.append('}');
        }

        private void array(Collection<?> list, int depth) throws IOException {
            if (list.isEmpty()) {
                out.append("[]");
                return;
            }
            out.append('[');
            boolean first = true;
            for (Object item : list) {
                out.append(first ? "\n" : ",\n");
                first = false;
                indent(depth + 1);
                value(item, depth + 1);
            }
            out.append('\n');
            indent(depth);
            out.append(']');
        }

        private void indent(int depth) throws IOException {
            for (int i = 0; i < depth; i++) {
                out.append("  ");
            }
        }

        private void string(String s) throws IOException {
            out.append('"');
            for (int i = 0; i < s.length(); i++) {
                char c = s.charAt(i);
                switch (c) {
                    case '"' -> out.append("\\\"");
                    case '\\' -> out.append("\\\\");
                    case '\b' -> out.append("\\b");
                    case '\f' -> out.append("\\f");
                    case '\n' -> out.append("\\n");
                    case '\r' -> out.append("\\r");
                    case '\t' -> out.append("\\t");
                    default -> {
                        if (c < 0x20) {
                            out.append(String.format("\\u%04x", (int) c));
                        } else if (Character.isHighSurrogate(c)
                                && i + 1 < s.length()
                                && Character.isLowSurrogate(s.charAt(i + 1))) {
                            out.append(c).append(s.charAt(i + 1));
                            i++;
                        } else if (Character.isSurrogate(c)) {
                            out.append('�');
                        } else {
                            out.append(c);
                        }
                    }
                }
            }
            out.append('"');
        }
    }
}
