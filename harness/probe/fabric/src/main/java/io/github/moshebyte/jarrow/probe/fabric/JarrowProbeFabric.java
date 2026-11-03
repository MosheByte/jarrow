// SPDX-License-Identifier: MIT
// Part of Jarrow (https://github.com/MosheByte/jarrow).
package io.github.moshebyte.jarrow.probe.fabric;

import io.github.moshebyte.jarrow.probe.LoaderInfo;
import io.github.moshebyte.jarrow.probe.LogCapture;
import io.github.moshebyte.jarrow.probe.ProbeDump;
import java.util.ArrayList;
import java.util.List;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;

/** The Fabric entry point: dumps on {@code ServerLifecycleEvents.SERVER_STARTED}. */
public final class JarrowProbeFabric implements ModInitializer {
    /** Public no-argument constructor, called by Fabric Loader. */
    public JarrowProbeFabric() {}

    @Override
    public void onInitialize() {
        LogCapture.install();
        ServerLifecycleEvents.SERVER_STARTED.register(server -> ProbeDump.onServerStarted(server, loaderInfo()));
    }

    private static LoaderInfo loaderInfo() {
        FabricLoader loader = FabricLoader.getInstance();
        List<LoaderInfo.Mod> mods = new ArrayList<>();
        for (ModContainer mod : loader.getAllMods()) {
            mods.add(new LoaderInfo.Mod(mod.getMetadata().getId(), mod.getMetadata().getVersion().getFriendlyString()));
        }
        return new LoaderInfo("fabric", version(loader, "fabricloader"), version(loader, "minecraft"), mods);
    }

    private static String version(FabricLoader loader, String id) {
        return loader.getModContainer(id)
                .map(mod -> mod.getMetadata().getVersion().getFriendlyString())
                .orElse("unknown");
    }
}
