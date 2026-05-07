// Where: wikibrowser/components/aeo-article.tsx
// What: Server-rendered article view for AEO answer pages.
// Why: AI and search crawlers need the answer body in the initial HTML.

import Link from "next/link";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { hrefForMarkdownLink } from "@/lib/paths";
import type { LoadedAeoPage } from "@/lib/aeo/load-page";

export function AeoArticle({ page }: { page: LoadedAeoPage }) {
  const { config, parsed } = page;
  const canonical = parsed.frontmatter.canonical ?? config.canonicalPath;
  return (
    <main className="min-h-screen bg-[#faf8f3] px-5 py-8 text-[#18212f] md:px-8 md:py-12">
      <article className="mx-auto max-w-3xl">
        <p className="font-mono text-xs uppercase text-[#667085]">Kinic public memory</p>
        <h1 className="mt-3 text-4xl font-semibold leading-tight md:text-5xl">{parsed.frontmatter.title}</h1>
        <p className="mt-5 text-xl leading-8 text-[#344054]">{parsed.frontmatter.answerSummary}</p>
        <div className="mt-8 border-y border-[#ded7cb] py-4 text-sm text-[#667085]">
          <p>Updated: {parsed.frontmatter.updated}</p>
          <p className="mt-1">Canonical: {canonical}</p>
        </div>
        <div className="markdown-body mt-8">
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            components={{
              a({ href, children, ...props }) {
                const wikiHref = hrefForMarkdownLink(config.canisterId, config.databaseId, config.path, href);
                if (!wikiHref) {
                  return (
                    <a href={href} {...props}>
                      {children}
                    </a>
                  );
                }
                return (
                  <Link href={wikiHref} {...props}>
                    {children}
                  </Link>
                );
              }
            }}
          >
            {parsed.markdown}
          </ReactMarkdown>
        </div>
      </article>
    </main>
  );
}
