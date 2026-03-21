/**
 * GraphAnimation — Interactive canvas-based knowledge graph visualization.
 * Ported from _archive/graph-animation.tsx as a React island (client:visible).
 */
import { useEffect, useRef } from "react";

interface Node {
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  label: string;
  color: string;
}

interface Edge {
  from: number;
  to: number;
}

const LABELS = [
  "Entity", "Document", "Chunk", "Query",
  "Graph", "Vector", "Pipeline", "LLM",
  "Storage", "Index", "Schema", "Edge",
  "Node", "Embed",
];

const COLORS = [
  "#3b82f6", "#60a5fa", "#93c5fd", "#2563eb",
  "#1d4ed8", "#3b82f6", "#60a5fa", "#93c5fd",
  "#2563eb", "#1d4ed8", "#3b82f6", "#60a5fa",
  "#93c5fd", "#2563eb",
];

const EDGES: Edge[] = [
  { from: 0, to: 1 }, { from: 0, to: 2 }, { from: 1, to: 3 },
  { from: 2, to: 4 }, { from: 3, to: 5 }, { from: 4, to: 6 },
  { from: 5, to: 7 }, { from: 6, to: 8 }, { from: 7, to: 9 },
  { from: 8, to: 10 }, { from: 9, to: 11 }, { from: 10, to: 12 },
  { from: 11, to: 13 }, { from: 12, to: 0 }, { from: 13, to: 1 },
  { from: 2, to: 7 }, { from: 4, to: 9 }, { from: 6, to: 11 },
];

export default function GraphAnimation() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

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

    const nodes: Node[] = LABELS.map((label, i) => {
      const angle = (i / LABELS.length) * Math.PI * 2;
      const r = 140 + Math.random() * 40;
      return {
        x: size / 2 + Math.cos(angle) * r,
        y: size / 2 + Math.sin(angle) * r,
        vx: (Math.random() - 0.5) * 0.3,
        vy: (Math.random() - 0.5) * 0.3,
        radius: 4 + Math.random() * 4,
        label,
        color: COLORS[i],
      };
    });

    let frame: number;
    let time = 0;

    const animate = () => {
      time += 0.016;
      ctx.clearRect(0, 0, size, size);

      // Update positions with gentle drift
      for (const node of nodes) {
        node.x += node.vx;
        node.y += node.vy;

        // Bounce off edges
        if (node.x < 40 || node.x > size - 40) node.vx *= -1;
        if (node.y < 40 || node.y > size - 40) node.vy *= -1;

        // Gentle center pull
        node.vx += (size / 2 - node.x) * 0.0001;
        node.vy += (size / 2 - node.y) * 0.0001;
      }

      // Draw edges
      for (const edge of EDGES) {
        const a = nodes[edge.from];
        const b = nodes[edge.to];
        ctx.beginPath();
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        ctx.strokeStyle = "rgba(59, 130, 246, 0.15)";
        ctx.lineWidth = 1;
        ctx.stroke();
      }

      // Draw nodes
      for (const node of nodes) {
        // Glow
        const pulse = Math.sin(time * 2 + node.x * 0.01) * 0.5 + 0.5;
        const glowRadius = node.radius + 8 + pulse * 4;
        const gradient = ctx.createRadialGradient(
          node.x, node.y, node.radius,
          node.x, node.y, glowRadius
        );
        gradient.addColorStop(0, node.color + "40");
        gradient.addColorStop(1, node.color + "00");
        ctx.beginPath();
        ctx.arc(node.x, node.y, glowRadius, 0, Math.PI * 2);
        ctx.fillStyle = gradient;
        ctx.fill();

        // Core
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
        ctx.fillStyle = node.color;
        ctx.fill();

        // Label
        ctx.fillStyle = "rgba(255, 255, 255, 0.6)";
        ctx.font = "10px Inter, sans-serif";
        ctx.textAlign = "center";
        ctx.fillText(node.label, node.x, node.y + node.radius + 14);
      }

      frame = requestAnimationFrame(animate);
    };

    animate();
    return () => cancelAnimationFrame(frame);
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="w-full h-full"
      aria-label="Animated knowledge graph visualization"
      role="img"
    />
  );
}
