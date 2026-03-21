"use client";

import { buttonVariants } from "@/components/ui/button";
import { ArrowLeft, Zap } from "lucide-react";
import Link from "next/link";

export default function NotFound() {
  return (
    <main className="flex min-h-[80vh] flex-col items-center justify-center px-4 text-center">
      <div className="mb-6 flex h-16 w-16 items-center justify-center rounded-full bg-accent/10">
        <Zap className="h-8 w-8 text-accent" />
      </div>
      <p className="mb-2 text-sm font-medium uppercase tracking-widest text-accent">
        404
      </p>
      <h1 className="mb-4 text-4xl font-bold tracking-tight sm:text-5xl">
        Page Not Found
      </h1>
      <p className="mb-8 max-w-md text-muted-foreground">
        The page you&apos;re looking for doesn&apos;t exist or has been moved.
        Let&apos;s get you back on track.
      </p>
      <div className="flex items-center gap-4">
        <Link href="/" className={buttonVariants({ className: "gap-2" })}>
          <ArrowLeft className="h-4 w-4" />
          Back to Home
        </Link>
        <Link
          href="/docs/"
          className={buttonVariants({ variant: "outline", className: "gap-2" })}
        >
          Documentation
        </Link>
      </div>
    </main>
  );
}
