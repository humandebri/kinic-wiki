export default function HomePage() {
  return (
    <main className="flex min-h-screen items-center justify-center px-6">
      <section className="max-w-xl rounded-2xl border border-line bg-paper p-8 shadow-sm">
        <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted">Kinic Wiki</p>
        <h1 className="mt-3 text-3xl font-semibold tracking-[-0.04em] text-ink">
          Open a wiki canister
        </h1>
        <p className="mt-4 text-sm leading-6 text-muted">
          Use <code>/w/&lt;canister-id&gt;/Wiki</code> to browse a read-only VFS tree.
        </p>
      </section>
    </main>
  );
}
