"use client";

import { useEffect, useRef } from "react";

/**
 * Soft cool-blue light field for the hero. Decorative only.
 * Reads theme from html.light so light mode gets a quieter wash.
 */
export function HeroField() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let raf = 0;
    let w = 0;
    let h = 0;
    let t = 0;
    let light = document.documentElement.classList.contains("light");

    type Node = { x: number; y: number; r: number; phase: number };
    let nodes: Node[] = [];

    function layout() {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = window.innerWidth;
      h = Math.min(window.innerHeight * 1.1, 1040);
      canvas!.width = Math.floor(w * dpr);
      canvas!.height = Math.floor(h * dpr);
      canvas!.style.width = `${w}px`;
      canvas!.style.height = `${h}px`;
      ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);

      nodes = [];
      const step = 44;
      const cols = Math.ceil(w / step) + 1;
      const rows = Math.ceil(h / step) + 1;
      for (let row = 0; row < rows; row++) {
        for (let col = 0; col < cols; col++) {
          if ((row * 3 + col * 5) % 4 === 0) continue;
          nodes.push({
            x: col * step + (row % 2) * (step * 0.35),
            y: row * step,
            r: 0.55 + ((col + row) % 4) * 0.2,
            phase: (col * 0.37 + row * 0.21) % (Math.PI * 2),
          });
        }
      }
    }

    function voidRgb() {
      return light ? "244, 245, 247" : "5, 5, 6";
    }

    function draw() {
      light = document.documentElement.classList.contains("light");
      const v = voidRgb();
      ctx!.clearRect(0, 0, w, h);

      const peak = light ? 0.18 : 0.22;
      const mid = light ? 0.08 : 0.1;

      const g = ctx!.createRadialGradient(
        w * 0.78,
        h * 0.08,
        0,
        w * 0.62,
        h * 0.42,
        Math.max(w, h) * 0.78
      );
      g.addColorStop(0, `rgba(120, 170, 255, ${peak})`);
      g.addColorStop(0.28, `rgba(70, 120, 220, ${mid})`);
      g.addColorStop(0.62, "rgba(30, 50, 100, 0.04)");
      g.addColorStop(1, `rgba(${v}, 0)`);
      ctx!.fillStyle = g;
      ctx!.fillRect(0, 0, w, h);

      const g2 = ctx!.createRadialGradient(
        w * 0.12,
        h * 0.92,
        0,
        w * 0.28,
        h * 0.72,
        w * 0.55
      );
      g2.addColorStop(0, light ? "rgba(50, 80, 150, 0.06)" : "rgba(50, 80, 150, 0.09)");
      g2.addColorStop(1, `rgba(${v}, 0)`);
      ctx!.fillStyle = g2;
      ctx!.fillRect(0, 0, w, h);

      for (const n of nodes) {
        const pulse = reduce
          ? 0.32
          : 0.18 + 0.32 * (0.5 + 0.5 * Math.sin(t * 0.7 + n.phase));
        const a = light ? pulse * 0.28 : pulse * 0.42;
        ctx!.beginPath();
        ctx!.arc(n.x, n.y, n.r, 0, Math.PI * 2);
        ctx!.fillStyle = `rgba(100, 130, 200, ${a})`;
        ctx!.fill();
      }

      const fade = ctx!.createLinearGradient(0, h * 0.5, 0, h);
      fade.addColorStop(0, `rgba(${v}, 0)`);
      fade.addColorStop(0.15, `rgba(${v}, 0.05)`);
      fade.addColorStop(0.3, `rgba(${v}, 0.15)`);
      fade.addColorStop(0.45, `rgba(${v}, 0.32)`);
      fade.addColorStop(0.6, `rgba(${v}, 0.52)`);
      fade.addColorStop(0.75, `rgba(${v}, 0.72)`);
      fade.addColorStop(0.88, `rgba(${v}, 0.88)`);
      fade.addColorStop(1, `rgba(${v}, 1)`);
      ctx!.fillStyle = fade;
      ctx!.fillRect(0, h * 0.5, w, h * 0.5);

      if (!reduce) {
        t += 0.016;
        raf = requestAnimationFrame(draw);
      }
    }

    const mo = new MutationObserver(() => {
      if (reduce) draw();
    });
    mo.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });

    layout();
    draw();
    window.addEventListener("resize", layout);

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", layout);
      mo.disconnect();
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      className="pointer-events-none absolute inset-x-0 top-0 z-0"
    />
  );
}
