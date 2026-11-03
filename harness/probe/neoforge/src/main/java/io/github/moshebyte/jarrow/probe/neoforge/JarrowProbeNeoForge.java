// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe.neoforge;

import io.github.moshebyte.jarrow.probe.LoaderInfo;
import io.github.moshebyte.jarrow.probe.LogCapture;
import io.github.moshebyte.jarrow.probe.ProbeDump;
import java.util.ArrayList;
import java.util.List;
import net.neoforged.api.distmarker.Dist;
import net.neoforged.fml.ModList;
import net.neoforged.fml.common.Mod;
import net.neoforged.neoforge.common.NeoForge;
import net.neoforged.neoforge.event.server.ServerStartedEvent;
import net.neoforged.neoforgespi.language.IModInfo;

/**
 * The NeoForge entry point: dumps on {@link ServerStartedEvent}. It is constructed on the dedicated
 * server only, which matches the Fabric probe's {@code "environment": "server"}.
 */
@Mod(value = "jarrow_probe", dist = Dist.DEDICATED_SERVER)
public final class JarrowProbeNeoForge {
    /** Called by FML when the mod is constructed. */
    public JarrowProbeNeoForge() {
        LogCapture.install();
        NeoForge.EVENT_BUS.addListener(ServerStartedEvent.class, JarrowProbeNeoForge::onServerStarted);
    }

    private static void onServerStarted(ServerStartedEvent event) {
        ProbeDump.onServerStarted(event.getServer(), loaderInfo());
    }

    private static LoaderInfo loaderInfo() {
        ModList modList = ModList.get();
        List<LoaderInfo.Mod> mods = new ArrayList<>();
        for (IModInfo mod : modList.getMods()) {
            mods.add(new LoaderInfo.Mod(mod.getModId(), mod.getVersion().toString()));
        }
        return new LoaderInfo("neoforge", version(modList, "neoforge"), version(modList, "minecraft"), mods);
    }

    private static String version(ModList modList, String id) {
        return modList.getModContainerById(id)
                .map(container -> container.getModInfo().getVersion().toString())
                .orElse("unknown");
    }
}
