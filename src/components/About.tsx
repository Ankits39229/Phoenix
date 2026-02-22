import { Github, Shield, Zap, HardDrive, ScanSearch, ExternalLink } from 'lucide-react'

const VERSION = '1.0.0'
const YEAR = new Date().getFullYear()

const features = [
  {
    icon: <Zap size={18} className="text-blue-500" />,
    title: 'Quick Scan',
    description: 'MFT-based recovery in seconds — ideal for recently deleted files.',
  },
  {
    icon: <ScanSearch size={18} className="text-purple-500" />,
    title: 'Deep Scan',
    description: 'Sector-by-sector carving recovers files with no MFT trace remaining.',
  },
  {
    icon: <HardDrive size={18} className="text-green-500" />,
    title: 'Multi-Drive',
    description: 'Works with HDDs, SSDs, USB drives, and custom folder paths.',
  },
  {
    icon: <Shield size={18} className="text-orange-500" />,
    title: 'Privacy First',
    description: 'Fully offline — no telemetry, no cloud uploads, ever.',
  },
]

const stack = [
  { name: 'Electron', color: 'bg-blue-100 text-blue-700' },
  { name: 'React', color: 'bg-cyan-100 text-cyan-700' },
  { name: 'TypeScript', color: 'bg-blue-100 text-blue-800' },
  { name: 'Rust', color: 'bg-orange-100 text-orange-700' },
  { name: 'Tailwind CSS', color: 'bg-teal-100 text-teal-700' },
  { name: 'framer-motion', color: 'bg-pink-100 text-pink-700' },
]

const About = () => {
  return (
    <div className="flex-1 overflow-y-auto">
      <div className="max-w-2xl mx-auto px-10 pt-24 pb-16">

        {/* Hero */}
        <div className="flex flex-col items-center text-center mb-12">
          <div className="w-20 h-20 rounded-3xl bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center shadow-lg mb-5">
            <HardDrive size={36} className="text-white" />
          </div>
          <h1 className="text-3xl font-semibold text-gray-700 mb-1">Data Recovery</h1>
          <p className="text-sm text-gray-400 mb-3">Version {VERSION}</p>
          <p className="text-sm text-gray-500 max-w-sm leading-relaxed">
            A fast, privacy-respecting desktop app to recover deleted files from any
            Windows drive — powered by a native Rust backend.
          </p>
        </div>

        {/* Features */}
        <section className="mb-10">
          <div className="flex items-center gap-2 mb-4">
            <h2 className="text-sm font-semibold text-blue-500 tracking-wide">Features</h2>
            <div className="flex-1 h-px bg-blue-100/60" />
          </div>
          <div className="grid grid-cols-2 gap-3">
            {features.map((f) => (
              <div
                key={f.title}
                className="bg-white/55 backdrop-blur-md rounded-2xl p-4 shadow-sm flex items-start gap-3"
              >
                <div className="shrink-0 w-9 h-9 rounded-xl bg-white/70 flex items-center justify-center shadow-sm">
                  {f.icon}
                </div>
                <div>
                  <p className="text-sm font-semibold text-gray-700 mb-0.5">{f.title}</p>
                  <p className="text-[11px] text-gray-400 leading-relaxed">{f.description}</p>
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* Tech Stack */}
        <section className="mb-10">
          <div className="flex items-center gap-2 mb-4">
            <h2 className="text-sm font-semibold text-blue-500 tracking-wide">Built With</h2>
            <div className="flex-1 h-px bg-blue-100/60" />
          </div>
          <div className="flex flex-wrap gap-2">
            {stack.map((s) => (
              <span
                key={s.name}
                className={`px-3 py-1 rounded-full text-xs font-medium ${s.color}`}
              >
                {s.name}
              </span>
            ))}
          </div>
        </section>

        {/* Links */}
        <section className="mb-10">
          <div className="flex items-center gap-2 mb-4">
            <h2 className="text-sm font-semibold text-blue-500 tracking-wide">Links</h2>
            <div className="flex-1 h-px bg-blue-100/60" />
          </div>
          <div className="flex flex-col gap-2">
            <a
              href="https://github.com"
              target="_blank"
              rel="noopener noreferrer"
              className="bg-white/55 backdrop-blur-md rounded-2xl px-4 py-3 shadow-sm flex items-center justify-between group hover:bg-white/75 transition-colors"
            >
              <div className="flex items-center gap-3">
                <Github size={18} className="text-gray-500" />
                <span className="text-sm text-gray-700 font-medium">Source Code</span>
              </div>
              <ExternalLink size={14} className="text-gray-300 group-hover:text-gray-500 transition-colors" />
            </a>
          </div>
        </section>

        {/* Footer */}
        <div className="text-center">
          <p className="text-[11px] text-gray-300">
            © {YEAR} Data Recovery · All rights reserved
          </p>
          <p className="text-[11px] text-gray-300 mt-1">
            Made with ♥ using Electron + Rust
          </p>
        </div>

      </div>
    </div>
  )
}

export default About
