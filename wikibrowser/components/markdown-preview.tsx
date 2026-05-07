"use client";

import Link from "next/link";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { hrefForMarkdownLink } from "@/lib/paths";

export function MarkdownPreview({
  canisterId,
  databaseId,
  nodePath,
  content
}: {
  canisterId: string;
  databaseId: string;
  nodePath: string;
  content: string;
}) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        a({ href, children, ...props }) {
          const wikiHref = hrefForMarkdownLink(canisterId, databaseId, nodePath, href);
          if (!wikiHref) {
            return <a href={href} {...props}>{children}</a>;
          }
          return <Link href={wikiHref} {...props}>{children}</Link>;
        }
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
