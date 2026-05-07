import type { MetadataRoute } from "next";
import { siteUrl } from "@/lib/aeo/site";

export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      {
        userAgent: "OAI-SearchBot",
        allow: ["/answers/", "/llms.txt", "/sitemap.xml"],
        disallow: ["/w/"]
      },
      {
        userAgent: "GPTBot",
        allow: ["/answers/", "/llms.txt", "/sitemap.xml"],
        disallow: ["/w/"]
      },
      {
        userAgent: "*",
        allow: ["/answers/", "/llms.txt", "/sitemap.xml"],
        disallow: ["/w/"]
      }
    ],
    sitemap: `${siteUrl()}/sitemap.xml`,
    host: siteUrl()
  };
}
