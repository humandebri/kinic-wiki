import type { MetadataRoute } from "next";
import { listAeoPages } from "@/lib/aeo/pages";
import { absoluteUrl } from "@/lib/aeo/site";

export default function sitemap(): MetadataRoute.Sitemap {
  return listAeoPages().map((page) => ({
    url: absoluteUrl(page.canonicalPath),
    changeFrequency: "weekly",
    priority: 0.8
  }));
}
