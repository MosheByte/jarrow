// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import net.minecraft.core.Holder;
import net.minecraft.core.HolderSet;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.ReloadableServerRegistries;
import net.minecraft.world.item.crafting.RecipeHolder;
import net.minecraft.world.level.storage.loot.LootTable;

/**
 * The game-version-specific calls of the dump, for Minecraft 1.21.1.
 *
 * <p>Every game version has its own copy of this class in {@code common/src/mc-<version>/java},
 * with the same methods. The loaders share it, so the dump stays identical across loaders.
 */
final class GameShim {
    private GameShim() {}

    /** The id of a resource key, such as {@code minecraft:stone}. */
    static String id(ResourceKey<?> key) {
        return key.location().toString();
    }

    /** Every tag of a registry: tag id to its member ids (unsorted; the caller sorts). */
    static <T> Map<String, List<String>> tags(Registry<T> registry) {
        Map<String, List<String>> out = new TreeMap<>(Json.KEY_ORDER);
        registry.getTags().forEach(pair -> out.put(pair.getFirst().location().toString(), members(pair.getSecond())));
        return out;
    }

    private static <T> List<String> members(HolderSet.Named<T> set) {
        List<String> ids = new ArrayList<>(set.size());
        for (Holder<T> holder : set) {
            ids.add(holder.unwrapKey().map(GameShim::id).orElse("<direct holder>"));
        }
        return ids;
    }

    /** The id of a recipe. In 1.21.1 a recipe holder's id is a location. */
    static String recipeId(RecipeHolder<?> holder) {
        return holder.id().toString();
    }

    /** Every loot table the server loaded, by id. */
    static Map<String, LootTable> lootTables(MinecraftServer server) {
        ReloadableServerRegistries.Holder registries = server.reloadableRegistries();
        Map<String, LootTable> out = new TreeMap<>(Json.KEY_ORDER);
        for (ResourceLocation id : registries.getKeys(Registries.LOOT_TABLE)) {
            out.put(id.toString(), registries.getLootTable(ResourceKey.create(Registries.LOOT_TABLE, id)));
        }
        return out;
    }
}
