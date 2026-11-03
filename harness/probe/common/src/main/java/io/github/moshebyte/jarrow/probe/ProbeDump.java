// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import com.google.gson.JsonElement;
import com.mojang.serialization.Codec;
import com.mojang.serialization.DataResult;
import com.mojang.serialization.DynamicOps;
import com.mojang.serialization.JsonOps;
import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.TreeMap;
import net.minecraft.advancements.Advancement;
import net.minecraft.advancements.AdvancementHolder;
import net.minecraft.commands.arguments.blocks.BlockStateParser;
import net.minecraft.core.Registry;
import net.minecraft.core.RegistryAccess;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.item.crafting.Recipe;
import net.minecraft.world.item.crafting.RecipeHolder;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.chunk.LevelChunk;
import net.minecraft.world.level.chunk.LevelChunkSection;
import net.minecraft.world.level.storage.loot.LootTable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Writes {@code jarrow-dump.json} for a started server and then stops it.
 *
 * <p>This is the whole probe. It is loader-neutral: each loader's entry class calls {@link
 * #onServerStarted} from its own "server started" event with a {@link LoaderInfo}. The format is
 * described in docs/probes.md; harness/dump-schema.json defines it for the harness. Calls whose
 * names differ between game versions go through {@link GameShim}, one copy per game version.
 *
 * <p>Nothing here throws out of {@link #onServerStarted}: a section that fails is left empty or
 * partial and its exception text goes into the dump's {@code "errors"} list, and the server is
 * halted in every case so the harness never waits on a server that will not stop.
 */
public final class ProbeDump {
    /** The dump's file name, inside the server directory. */
    public static final String FILE_NAME = "jarrow-dump.json";

    /** The dump format version. */
    public static final int SCHEMA = 1;

    /** Chunk coordinates (not block coordinates) of the worldgen sample, in this order. */
    static final int[][] CHUNKS = {{0, 0}, {8, 8}, {-8, 8}, {8, -8}};

    private static final Logger LOGGER = LoggerFactory.getLogger("jarrow_probe");

    private ProbeDump() {}

    /** Dumps the server into {@code <server directory>/jarrow-dump.json}, then halts it. */
    public static void onServerStarted(MinecraftServer server, LoaderInfo loader) {
        try {
            Path out = server.getServerDirectory().resolve(FILE_NAME);
            // A dump left over from an earlier run must never pass for this one.
            Files.deleteIfExists(out);
            LOGGER.info("Jarrow probe: dumping the server to {}", out.toAbsolutePath());
            Map<String, Object> dump = collect(server, loader);
            write(out, dump);
            @SuppressWarnings("unchecked")
            List<String> errors = (List<String>) dump.get("errors");
            LOGGER.info("Jarrow probe: wrote {} ({} dump errors)", out.toAbsolutePath(), errors.size());
        } catch (IOException | RuntimeException | LinkageError e) {
            LOGGER.error("Jarrow probe: could not write {}", FILE_NAME, e);
        } finally {
            LOGGER.info("Jarrow probe: stopping the server");
            server.halt(false);
        }
    }

    /** Builds the dump as a tree of maps, lists and Gson elements. */
    static Map<String, Object> collect(MinecraftServer server, LoaderInfo loader) {
        List<String> errors = new ArrayList<>();
        Map<String, Object> dump = new TreeMap<>(Json.KEY_ORDER);
        dump.put("schema", SCHEMA);
        dump.put("loader", loader.loader());
        dump.put("loader_version", loader.loaderVersion());
        dump.put("game_version", loader.gameVersion());
        dump.put("mods", mods(loader));

        RegistryAccess access = server.registryAccess();
        dump.put("registries", guard("registries", errors, () -> registries(access, errors)));
        dump.put("tags", guard("tags", errors, () -> tags(access, errors)));
        dump.put("recipes", guard("recipes", errors, () -> recipes(server, errors)));
        dump.put("loot_tables", guard("loot_tables", errors, () -> lootTables(server, errors)));
        dump.put("advancements", guard("advancements", errors, () -> advancements(server, errors)));
        dump.put("worldgen", worldgen(server, errors));
        dump.put("log", LogCapture.snapshot(errors));
        dump.put("errors", errors);
        return dump;
    }

    private static List<Object> mods(LoaderInfo loader) {
        List<Object> mods = new ArrayList<>();
        for (LoaderInfo.Mod mod : loader.sortedMods()) {
            Map<String, Object> entry = new TreeMap<>(Json.KEY_ORDER);
            entry.put("id", mod.id());
            entry.put("version", mod.version());
            mods.add(entry);
        }
        return mods;
    }

    private interface Section {
        Map<String, Object> get();
    }

    /** Runs one section; a failure leaves it empty and records the exception. */
    private static Map<String, Object> guard(String name, List<String> errors, Section body) {
        try {
            return body.get();
        } catch (RuntimeException | LinkageError e) {
            errors.add(name + ": " + e);
            LOGGER.error("Jarrow probe: section {} failed", name, e);
            return new TreeMap<>(Json.KEY_ORDER);
        }
    }

    /** Every registry in the server's registry access, static and dynamic, in iteration order. */
    static Map<String, Object> registries(RegistryAccess access, List<String> errors) {
        Map<String, Object> out = new TreeMap<>(Json.KEY_ORDER);
        access.registries().forEach(entry -> {
            String id = GameShim.id(entry.key());
            try {
                out.put(id, entryIds(entry.value()));
            } catch (RuntimeException | LinkageError e) {
                errors.add("registries: " + id + ": " + e);
            }
        });
        return out;
    }

    private static <T> List<String> entryIds(Registry<T> registry) {
        List<String> ids = new ArrayList<>(registry.size());
        for (T value : registry) {
            ids.add(String.valueOf(registry.getKey(value)));
        }
        return ids;
    }

    /** Every tag of every registry: tag id to its sorted entry ids. */
    static Map<String, Object> tags(RegistryAccess access, List<String> errors) {
        Map<String, Object> out = new TreeMap<>(Json.KEY_ORDER);
        access.registries().forEach(entry -> {
            String id = GameShim.id(entry.key());
            try {
                Map<String, List<String>> tags = GameShim.tags(entry.value());
                for (List<String> members : tags.values()) {
                    members.sort(Json.KEY_ORDER);
                }
                out.put(id, tags);
            } catch (RuntimeException | LinkageError e) {
                errors.add("tags: " + id + ": " + e);
            }
        });
        return out;
    }

    private static DynamicOps<JsonElement> jsonOps(MinecraftServer server) {
        return server.registryAccess().createSerializationContext(JsonOps.INSTANCE);
    }

    static Map<String, Object> recipes(MinecraftServer server, List<String> errors) {
        DynamicOps<JsonElement> ops = jsonOps(server);
        Map<String, Object> out = new TreeMap<>(Json.KEY_ORDER);
        for (RecipeHolder<?> holder : server.getRecipeManager().getRecipes()) {
            Recipe<?> recipe = holder.value();
            encode("recipes", GameShim.recipeId(holder), Recipe.CODEC, recipe, ops, out, errors);
        }
        return out;
    }

    static Map<String, Object> lootTables(MinecraftServer server, List<String> errors) {
        DynamicOps<JsonElement> ops = jsonOps(server);
        Map<String, Object> out = new TreeMap<>(Json.KEY_ORDER);
        for (Map.Entry<String, LootTable> table : GameShim.lootTables(server).entrySet()) {
            encode("loot_tables", table.getKey(), LootTable.DIRECT_CODEC, table.getValue(), ops, out, errors);
        }
        return out;
    }

    static Map<String, Object> advancements(MinecraftServer server, List<String> errors) {
        DynamicOps<JsonElement> ops = jsonOps(server);
        Map<String, Object> out = new TreeMap<>(Json.KEY_ORDER);
        for (AdvancementHolder holder : server.getAdvancements().getAllAdvancements()) {
            encode("advancements", String.valueOf(holder.id()), Advancement.CODEC, holder.value(), ops, out, errors);
        }
        return out;
    }

    private static <A> void encode(
            String section,
            String id,
            Codec<A> codec,
            A value,
            DynamicOps<JsonElement> ops,
            Map<String, Object> out,
            List<String> errors) {
        try {
            DataResult<JsonElement> result = codec.encodeStart(ops, value);
            Optional<JsonElement> json = result.result();
            if (json.isPresent()) {
                out.put(id, json.get());
            } else {
                String message = result.error().map(DataResult.Error::message).orElse("the codec returned nothing");
                errors.add(section + ": " + id + ": " + message);
            }
        } catch (RuntimeException | LinkageError e) {
            errors.add(section + ": " + id + ": " + e);
        }
    }

    /** The seed and a block-state histogram over {@link #CHUNKS} in the overworld. */
    static Map<String, Object> worldgen(MinecraftServer server, List<String> errors) {
        Map<String, Object> out = new TreeMap<>(Json.KEY_ORDER);
        List<Object> chunks = new ArrayList<>();
        for (int[] chunk : CHUNKS) {
            chunks.add(List.of(chunk[0], chunk[1]));
        }
        out.put("chunks", chunks);
        Map<String, Long> histogram = new TreeMap<>(Json.KEY_ORDER);
        out.put("block_histogram", histogram);
        try {
            ServerLevel level = server.overworld();
            out.put("seed", level.getSeed());
            Map<BlockState, long[]> counts = new IdentityHashMap<>();
            for (int[] pos : CHUNKS) {
                LevelChunk chunk = level.getChunk(pos[0], pos[1]);
                for (LevelChunkSection section : chunk.getSections()) {
                    for (int y = 0; y < 16; y++) {
                        for (int z = 0; z < 16; z++) {
                            for (int x = 0; x < 16; x++) {
                                counts.computeIfAbsent(section.getBlockState(x, y, z), s -> new long[1])[0]++;
                            }
                        }
                    }
                }
            }
            for (Map.Entry<BlockState, long[]> count : counts.entrySet()) {
                histogram.merge(BlockStateParser.serialize(count.getKey()), count.getValue()[0], Long::sum);
            }
        } catch (RuntimeException | LinkageError e) {
            errors.add("worldgen: " + e);
            LOGGER.error("Jarrow probe: section worldgen failed", e);
        }
        if (!out.containsKey("seed")) {
            out.put("seed", null);
        }
        return out;
    }

    static void write(Path out, Map<String, Object> dump) throws IOException {
        Path tmp = out.resolveSibling(out.getFileName() + ".tmp");
        try (BufferedWriter writer = Files.newBufferedWriter(tmp, StandardCharsets.UTF_8)) {
            Json.write(dump, writer);
        }
        try {
            Files.move(tmp, out, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } catch (AtomicMoveNotSupportedException e) {
            Files.move(tmp, out, StandardCopyOption.REPLACE_EXISTING);
        }
    }
}
