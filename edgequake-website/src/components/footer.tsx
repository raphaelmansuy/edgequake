import { Zap } from "lucide-react";
import Link from "next/link";

const footerLinks = {
  Product: [
    { href: "/docs/", label: "Getting Started" },
    { href: "/demo/", label: "Live Demo" },
    { href: "/ecosystem/", label: "Ecosystem" },
    { href: "/enterprise/", label: "Enterprise" },
  ],
  Developers: [
    { href: "/docs/", label: "Documentation" },
    { href: "https://github.com/raphaelmansuy/edgequake/tree/main/docs/concepts", label: "Core Concepts" },
    { href: "https://github.com/raphaelmansuy/edgequake/tree/main/docs/api-reference", label: "API Reference" },
    { href: "https://crates.io/crates/edgequake-llm", label: "crates.io" },
  ],
  Community: [
    { href: "https://github.com/raphaelmansuy/edgequake", label: "GitHub" },
    { href: "https://github.com/raphaelmansuy/edgequake/issues", label: "Issues" },
    { href: "https://github.com/raphaelmansuy/edgequake/discussions", label: "Discussions" },
    { href: "https://github.com/raphaelmansuy/edgequake/blob/main/CHANGELOG.md", label: "Changelog" },
  ],
  Company: [
    { href: "/contact/", label: "Contact" },
    { href: "https://elitizon.com", label: "Elitizon" },
    { href: "https://github.com/raphaelmansuy/edgequake/blob/main/LICENSE", label: "License (Apache 2.0)" },
  ],
};

export function Footer() {
  return (
    <footer className="border-t border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-20">
        <div className="grid grid-cols-2 md:grid-cols-5 gap-8">
          {/* Brand Column */}
          <div className="col-span-2 md:col-span-1">
            <Link href="/" className="flex items-center gap-2 font-bold text-lg tracking-tight mb-4">
              <Zap className="h-5 w-5 text-accent" />
              <span>EdgeQuake</span>
            </Link>
            <p className="text-sm text-muted-foreground max-w-xs leading-relaxed">
              Graph-RAG framework built in Rust for production workloads. Apache 2.0 licensed.
            </p>
          </div>

          {/* Link Columns */}
          {Object.entries(footerLinks).map(([title, links]) => (
            <div key={title}>
              <h3 className="text-sm font-semibold mb-3">{title}</h3>
              <ul className="space-y-2">
                {links.map((link) => (
                  <li key={link.href}>
                    {link.href.startsWith("http") ? (
                      <a
                        href={link.href}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                      >
                        {link.label}
                      </a>
                    ) : (
                      <Link
                        href={link.href}
                        className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                      >
                        {link.label}
                      </Link>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        {/* Bottom Bar */}
        <div className="mt-12 pt-8 border-t border-border flex flex-col sm:flex-row items-center justify-between gap-4">
          <p className="text-xs text-muted-foreground">
            &copy; {new Date().getFullYear()} EdgeQuake &middot; Built by{" "}
            <a href="https://elitizon.com" className="hover:text-foreground transition-colors" target="_blank" rel="noopener noreferrer">
              Elitizon
            </a>{" "}
            &middot; Apache 2.0
          </p>
          <div className="flex items-center gap-4">
            <a
              href="https://github.com/raphaelmansuy/edgequake"
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              GitHub
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
}
