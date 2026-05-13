"use client";

// Dashboard action buttons: keep ACL button feedback consistent during writes.

import { Loader2 } from "lucide-react";
import type { ReactNode } from "react";

export function ActionButton({
  children,
  dataTid,
  disabled = false,
  loading = false,
  loadingLabel,
  onClick,
  size = "normal",
  type = "button",
  variant
}: {
  children: ReactNode;
  dataTid?: string;
  disabled?: boolean;
  loading?: boolean;
  loadingLabel?: string;
  onClick?: () => void;
  size?: "normal" | "compact";
  type?: "button" | "submit";
  variant: "primary" | "secondary" | "danger";
}) {
  const baseClass =
    "inline-flex min-w-[96px] items-center justify-center gap-2 rounded-lg border text-sm transition duration-150 ease-out active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent";
  const sizeClass = size === "compact" ? "px-3 py-1.5" : "px-3 py-2";
  const variantClass = buttonVariantClass(variant, loading);
  return (
    <button aria-busy={loading || undefined} className={`${baseClass} ${sizeClass} ${variantClass}`} data-tid={dataTid} disabled={disabled} type={type} onClick={onClick}>
      {loading ? <Loader2 aria-hidden className="animate-spin" size={15} /> : null}
      <span>{loading && loadingLabel ? loadingLabel : children}</span>
    </button>
  );
}

function buttonVariantClass(variant: "primary" | "secondary" | "danger", loading: boolean): string {
  if (variant === "danger") return loading ? "border-red-800 bg-red-800 text-white shadow-sm ring-2 ring-red-200" : "border-red-700 bg-red-700 text-white hover:bg-red-800";
  if (variant === "primary") return loading ? "border-blue-800 bg-blue-800 text-white shadow-sm ring-2 ring-blue-200" : "border-accent bg-accent font-medium text-white hover:bg-blue-700";
  return loading ? "border-accent bg-blue-50 text-accent shadow-sm ring-2 ring-blue-100" : "border-line bg-white text-ink hover:border-accent hover:bg-blue-50";
}
