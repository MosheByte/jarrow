// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import net.minecraft.core.Holder;
import net.minecraft.core.HolderLookup;
import net.minecraft.core.HolderSet;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.MinecraftServer;
import net.minecraft.world.item.crafting.RecipeHolder;
import net.minecraft.world.level.storage.loot.LootTable;

/**
 * The game-version-specific calls of the dump, for Minecraft 1.21.11.
 *
 * <p>Differences from 1.21.1 that this class absorbs: Mojang renamed {@code ResourceLocation} to
 * {@code Identifier} (and {@code ResourceKey.location()} to {@code identifier()}), {@code
 * Registry.getTags()} streams named holder sets instead of pairs, recipe holder ids are resource
 * keys, and loot tables are listed through the reloadable registries' lookup.
 */
final class GameShim {
    private GameShim() {}

    /** The id of a resource key, such as {@code minecraft:stone}. */
    static String id(ResourceKey<?> key) {
        return key.identifier().toString();
    }

    /** Every tag of a registry: tag id to its member ids (unsorted; the caller sorts). */
    static <T> Map<String, List<String>> tags(Registry<T> registry) {
        Map<String, List<String>> out = new TreeMap<>(Json.KEY_ORDER);
        registry.getTags().forEach(named -> out.put(named.key().location().toString(), members(named)));
        return out;
    }

    private static <T> List<String> members(HolderSet.Named<T> set) {
        List<String> ids = new ArrayList<>(set.size());
        for (Holder<T> holder : set) {
            ids.add(holder.unwrapKey().map(GameShim::id).orElse("<direct holder>"));
        }
        return ids;
    }

    /** The id of a recipe. Since 1.21.2 a recipe holder's id is a resource key. */
    static String recipeId(RecipeHolder<?> holder) {
        return id(holder.id());
    }

    /** Every loot table the server loaded, by id. */
    static Map<String, LootTable> lootTables(MinecraftServer server) {
        HolderLookup.RegistryLookup<LootTable> lookup =
                server.reloadableRegistries().lookup().lookupOrThrow(Registries.LOOT_TABLE);
        Map<String, LootTable> out = new TreeMap<>(Json.KEY_ORDER);
        lookup.listElements().forEach(table -> out.put(id(table.key()), table.value()));
        return out;
    }
}
