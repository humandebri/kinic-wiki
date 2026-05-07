import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { AeoArticle } from "@/components/aeo-article";
import { absoluteUrl } from "@/lib/aeo/site";
import { loadAeoPage } from "@/lib/aeo/load-page";

export const runtime = "nodejs";
export const revalidate = 3600;

type PageProps = {
  params: Promise<{
    slug: string;
  }>;
};

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = await params;
  const page = await loadAeoPage(slug);
  if (!page) {
    return {};
  }
  const canonicalPath = page.parsed.frontmatter.canonical ?? page.config.canonicalPath;
  const url = absoluteUrl(canonicalPath);
  return {
    title: page.parsed.frontmatter.title,
    description: page.parsed.frontmatter.description,
    alternates: {
      canonical: url
    },
    openGraph: {
      type: "article",
      title: page.parsed.frontmatter.title,
      description: page.parsed.frontmatter.description,
      url,
      locale: page.config.locale
    }
  };
}

export default async function AnswerPage({ params }: PageProps) {
  const { slug } = await params;
  const page = await loadAeoPage(slug);
  if (!page) {
    notFound();
  }
  const canonicalPath = page.parsed.frontmatter.canonical ?? page.config.canonicalPath;
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "Article",
    headline: page.parsed.frontmatter.title,
    description: page.parsed.frontmatter.description,
    dateModified: page.parsed.frontmatter.updated,
    mainEntityOfPage: absoluteUrl(canonicalPath),
    about: page.parsed.frontmatter.entities
  };
  return (
    <>
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }} />
      <AeoArticle page={page} />
    </>
  );
}
