import { Check, ExternalLink } from "lucide-react";
import { api } from "../api";
import type { AiStatus } from "../types";
import { Button, Modal } from "../ui";

export function OllamaGuide({ status, onClose, onRecheck }: { status: AiStatus | null; onClose: () => void; onRecheck: () => void }) {
  const model = status?.model ?? "llama3.2:3b";
  const step = !status?.reachable ? 1 : !status.model_available ? 2 : 3;
  const Step = ({ n, title, children }: { n: number; title: string; children?: React.ReactNode }) => {
    const done = step > n;
    const current = step === n;
    return (
      <li className={`flex gap-4 rounded-xl p-4 transition-colors ${current ? "bg-tint" : ""}`}>
        <span
          className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full font-mono text-sm font-medium ${
            done ? "bg-brand text-ink" : current ? "bg-surface text-accent shadow-soft" : "bg-bg text-ink-3"
          }`}
        >
          {done ? <Check size={16} strokeWidth={2.5} /> : n}
        </span>
        <div className="min-w-0 pt-1">
          <div className={`text-[15px] font-semibold ${current || done ? "text-ink" : "text-ink-2"}`}>{title}</div>
          {children && <div className="mt-1.5 text-sm leading-6 text-ink-2">{children}</div>}
        </div>
      </li>
    );
  };
  return (
    <Modal title="Smart AI sorting (optional)" onClose={onClose}>
      <p className="mb-5 text-[15px] leading-6 text-ink-2">
        Rules alone work well. A free local model can also read the first lines of unclear files, like <span className="font-mono text-ink">scan0042.pdf</span>, to sort them better.{" "}
        <span className="font-semibold text-ink">It runs on your computer. Nothing is uploaded.</span>
      </p>
      <ol className="space-y-1">
        <Step n={1} title="Install Ollama (free)">
          <Button variant={step === 1 ? "primary" : "outline"} size="sm" className="mt-1" onClick={() => api.openOllamaDownload()}>
            Open ollama.com/download <ExternalLink size={14} />
          </Button>
          <div className="mt-2">Run the installer, then come back here.</div>
        </Step>
        <Step n={2} title="Download a small model (about 2 GB, once)">
          Open PowerShell (press Start, type <em>PowerShell</em>) and run:
          <code className="mt-2 block select-all rounded-lg bg-bg px-3 py-2 font-mono text-[13px] text-ink">ollama pull {model}</code>
        </Step>
        <Step n={3} title="That's it. tidydrop detects it automatically." />
      </ol>
      {status && <p className="mt-5 text-sm text-ink-3">Status: {status.message}</p>}
      <div className="mt-6 flex justify-end gap-2">
        <Button variant="quiet" onClick={onClose}>
          Close
        </Button>
        <Button variant={step === 1 ? "outline" : "primary"} onClick={onRecheck}>
          Check again
        </Button>
      </div>
    </Modal>
  );
}
