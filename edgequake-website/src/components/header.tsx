"use client";

import { ThemeToggle } from "@/components/theme-toggle";
import { Button, buttonVariants } from "@/components/ui/button";
import { Github, Menu, X, Zap } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

const navLinks = [
  { href: "/docs/", label: "Docs" },
  { href: "/demo/", label: "Demo" },
  { href: "/ecosystem/", label: "Ecosystem" },
  { href: "/enterprise/", label: "Enterprise" },
];

export function Header() {
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <header className="sticky top-0 z-50 w-full border-b border-border/50 bg-background/80 backdrop-blur-xl">
      <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
        {/* Logo */}
        <Link href="/" className="flex items-center gap-2 font-bold text-lg tracking-tight">
          <Zap className="h-5 w-5 text-accent" />
          <span>EdgeQuake</span>
        </Link>

        {/* Desktop Nav */}
        <nav className="hidden md:flex items-center gap-1">
          {navLinks.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className="px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
            >
              {link.label}
            </Link>
          ))}
        </nav>

        {/* Desktop Actions */}
        <div className="hidden md:flex items-center gap-2">
          <ThemeToggle />
          <a
            href="https://github.com/raphaelmansuy/edgequake"
            target="_blank"
            rel="noopener noreferrer"
            aria-label="GitHub"
            className={buttonVariants({ variant: "ghost", size: "icon" })}
          >
            <Github className="h-4 w-4" />
          </a>
          <Link href="/docs/" className={buttonVariants({ size: "sm" })}>
            Get Started
          </Link>
        </div>

        {/* Mobile Toggle */}
        <div className="flex md:hidden items-center gap-2">
          <ThemeToggle />
          <Button variant="ghost" size="icon" onClick={() => setMobileOpen(!mobileOpen)} aria-label="Menu">
            {mobileOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
          </Button>
        </div>
      </div>

      {/* Mobile Menu */}
      {mobileOpen && (
        <div className="md:hidden border-t border-border bg-background">
          <nav className="flex flex-col p-4 gap-1">
            {navLinks.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                onClick={() => setMobileOpen(false)}
                className="px-3 py-2 text-sm font-medium text-muted-foreground hover:text-foreground rounded-md hover:bg-muted"
              >
                {link.label}
              </Link>
            ))}
            <div className="mt-2 pt-2 border-t border-border flex gap-2">
              <a
                href="https://github.com/raphaelmansuy/edgequake"
                target="_blank"
                rel="noopener noreferrer"
                className={buttonVariants({ variant: "outline", size: "sm", className: "flex-1 gap-2" })}
              >
                <Github className="h-4 w-4" /> GitHub
              </a>
              <Link
                href="/docs/"
                onClick={() => setMobileOpen(false)}
                className={buttonVariants({ size: "sm", className: "flex-1" })}
              >
                Get Started
              </Link>
            </div>
          </nav>
        </div>
      )}
    </header>
  );
}
