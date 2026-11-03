import { docsLoader, i18nLoader } from '@astrojs/starlight/loaders';
import { docsSchema, i18nSchema } from '@astrojs/starlight/schema';
import { defineCollection } from 'astro:content';

export const collections = {
  // Docs live in src/content/docs/docs/ so Starlight serves them under /docs/.
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
  // UI string overrides for Starlight (src/content/i18n/en.json).
  i18n: defineCollection({ loader: i18nLoader(), schema: i18nSchema() }),
};
