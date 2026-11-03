// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

/**
 * What only the loader knows: its id, its version, the game version and the loaded mods. Each
 * loader's entry class builds one of these; everything else in the dump comes from the game.
 *
 * @param loader the loader id: {@code fabric}, {@code neoforge}, {@code forge} or {@code quilt}
 * @param loaderVersion the loader's own version, as the loader reports it
 * @param gameVersion the Minecraft version, as the loader reports it
 * @param mods every loaded mod, in any order
 */
public record LoaderInfo(String loader, String loaderVersion, String gameVersion, List<Mod> mods) {
    /**
     * One loaded mod.
     *
     * @param id the mod id
     * @param version the mod's version string
     */
    public record Mod(String id, String version) {}

    /** The mods sorted by id, then version, as the dump lists them. */
    public List<Mod> sortedMods() {
        List<Mod> sorted = new ArrayList<>(mods);
        sorted.sort(Comparator.comparing(Mod::id, Json.KEY_ORDER).thenComparing(Mod::version, Json.KEY_ORDER));
        return sorted;
    }
}
