"use client";

import { useEffect, useRef } from "react";

interface Node {
  x: number;
  y: number;
  vx: number;
  vy: number;
  label: string;
  color: string;
  radius: number;
}

interface Edge {
  from: number;
  to: number;
}

const labels = [
  "Person", "Org", "Concept", "Document", "Entity",
  "Query", "Graph", "Node", "Edge", "RAG",
  "Embed", "LLM", "Rust", "API",
];

const colors = ["#3B82F6", "#2563EB", "#60A5FA", "#3B82F6", "#2563EB", "#60A5FA"];

export function GraphAnimation() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const size = 480;
    canvas.width = size * dpr;
    canvas.height = size * dpr;
    canvas.style.width = `${size}px`;
    canvas.style.height = `${size}px`;
    ctx.scale(dpr, dpr);

    // Create nodes
    const nodes: Node[] = labels.map((label, i) => ({
      x: size / 2 + Math.cos((i / labels.length) * Math.PI * 2) * (120 + Math.random() * 60),
      y: size / 2 + Math.sin((i / labels.length) * Math.PI * 2) * (120 + Math.random() * 60),
      vx: (Math.random() - 0.5) * 0.3,
      vy: (Math.random() - 0.5) * 0.3,
      label,
      color: colors[i % colors.length],
      radius: 18 + Math.random() * 10,
    }));

    // Create edges (sparse random connections)
    const edges: Edge[] = [];
    for (let i = 0; i < nodes.length; i++) {
      const numConnections = 1 + Math.floor(Math.random() * 2);
      for (let c = 0; c < numConnections; c++) {
        const to = (i + 1 + Math.floor(Math.random() * (nodes.length - 2))) % nodes.length;
        edges.push({ from: i, to });
      }
    }

    let time = 0;

    function animate() {
      if (!ctx) return;
      time += 0.01;
      ctx.clearRect(0, 0, size, size);

      // Update positions with gentle movement
      for (const node of nodes) {
        node.x += node.vx;
        node.y += node.vy;

        // Bounce off edges
        if (node.x < 40 || node.x > size - 40) node.vx *= -1;
        if (node.y < 40 || node.y > size - 40) node.vy *= -1;

        // Gentle center gravity
        node.vx += (size / 2 - node.x) * 0.0002;
        node.vy += (size / 2 - node.y) * 0.0002;
      }

      // Draw edges
      for (const edge of edges) {
        const from = nodes[edge.from];
        const to = nodes[edge.to];
        const pulse = 0.15 + 0.1 * Math.sin(time * 2 + edge.from);
        ctx.beginPath();
        ctx.moveTo(from.x, from.y);
        ctx.lineTo(to.x, to.y);
        ctx.strokeStyle = `rgba(59, 130, 246, ${pulse})`;
        ctx.lineWidth = 1;
        ctx.stroke();
      }

      // Draw nodes
      for (const node of nodes) {
        const pulse = 1 + 0.05 * Math.sin(time * 3 + node.x * 0.01);
        const r = node.radius * pulse;

        // Glow
        ctx.beginPath();
        ctx.arc(node.x, node.y, r + 4, 0, Math.PI * 2);
        ctx.fillStyle = `${node.color}15`;
        ctx.fill();

        // Node circle
        ctx.beginPath();
        ctx.arc(node.x, node.y, r, 0, Math.PI * 2);
        ctx.fillStyle = `${node.color}20`;
        ctx.strokeStyle = `${node.color}60`;
        ctx.lineWidth = 1.5;
        ctx.fill();
        ctx.stroke();

        // Label
        ctx.fillStyle = getComputedStyle(document.documentElement)
          .getPropertyValue("--foreground")
          .trim() || "#F8F9FA";
        ctx.font = "10px system-ui, sans-serif";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(node.label, node.x, node.y);
      }

      animRef.current = requestAnimationFrame(animate);
    }

    animate();

    return () => cancelAnimationFrame(animRef.current);
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="w-full h-full"
      style={{ maxWidth: 480, maxHeight: 480 }}
    />
  );
}
