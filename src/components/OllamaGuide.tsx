import { api } from "../api";
import type { AiStatus } from "../types";
import { Button, Modal } from "../ui";

export function OllamaGuide({ status, onClose, onRecheck }: { status: AiStatus | null; onClose: () => void; onRecheck: () => void }) {
  const model = status?.model ?? "llama3.2:3b";
  const step = !status?.reachable ? 1 : !status.model_available ? 2 : 3;
  const Step = ({ n, title, children }: { n: number; title: string; children: React.ReactNode }) => (
    <li className={`flex gap-3 rounded-xl p-3 ${step === n ? "bg-brand-50 ring-1 ring-brand-200 dark:bg-brand-900/30 dark:ring-brand-800" : ""}`}>
      <span
        className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-sm font-bold ${
          step > n ? "bg-brand-500 text-white" : "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200"
        }`}
      >
        {step > n ? "✓" : n}
      </span>
      <div className="text-sm">
        <div className="font-semibold">{title}</div>
        <div className="mt-1 text-slate-600 dark:text-slate-300">{children}</div>
      </div>
    </li>
  );
  return (
    <Modal title="Turn on smart AI sorting (optional)" onClose={onClose}>
      <p className="mb-4 text-sm text-slate-600 dark:text-slate-300">
        tidydrop works great with rules alone. With a free local AI model it can also read the first lines of unclear files
        (like <em>scan0042.pdf</em>) to sort them better. <strong>The AI runs on your computer. Nothing is uploaded.</strong>
      </p>
      <ol className="space-y-2">
        <Step n={1} title="Install Ollama (free)">
          <Button variant="primary" className="mt-1" onClick={() => api.openOllamaDownload()}>
            Open ollama.com/download
          </Button>
          <div className="mt-1 text-xs">Run the installer, then come back here.</div>
        </Step>
        <Step n={2} title="Download a small model (about 2 GB, once)">
          Open a terminal (Windows: press Start, type <em>PowerShell</em>) and run:
          <code className="mt-1 block select-all rounded-lg bg-slate-900 px-3 py-2 font-mono text-xs text-brand-200">ollama pull {model}</code>
        </Step>
        <Step n={3} title="Done">That's it. tidydrop detects it automatically.</Step>
      </ol>
      {status && <p className="mt-4 text-xs text-slate-500">Status: {status.message}</p>}
      <div className="mt-5 flex justify-end gap-2">
        <Button variant="ghost" onClick={onClose}>
          Close
        </Button>
        <Button variant="primary" onClick={onRecheck}>
          Check again
        </Button>
      </div>
    </Modal>
  );
}
