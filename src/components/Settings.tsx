import { useState } from 'react'
import { ScanSearch, Trash2, FolderOpen, Monitor, Bell, BellOff, Shield, Check } from 'lucide-react'
import { useTheme, type Theme } from '../context/ThemeContext'

const Settings = () => {
  const { theme, setTheme } = useTheme()
  const [deepScanLimit, setDeepScanLimit] = useState(500)
  const [quickScanLimit, setQuickScanLimit] = useState(100)
  const [notifications, setNotifications] = useState(true)
  const [autoOpenFolder, setAutoOpenFolder] = useState(false)
  const [skipSystemFiles, setSkipSystemFiles] = useState(true)

  const SectionTitle = ({ icon, label }: { icon: React.ReactNode; label: string }) => (
    <div className="flex items-center gap-2 mb-4">
      <div className="text-blue-500">{icon}</div>
      <h2 className="text-sm font-semibold text-gray-700 tracking-wide">{label}</h2>
      <div className="flex-1 h-px bg-blue-100/60" />
    </div>
  )

  const Toggle = ({
    checked,
    onChange,
    label,
    description,
  }: {
    checked: boolean
    onChange: (v: boolean) => void
    label: string
    description?: string
  }) => (
    <div className="flex items-center justify-between py-3 px-4 rounded-xl bg-white/50 backdrop-blur-sm">
      <div>
        <p className="text-sm font-medium text-gray-700">{label}</p>
        {description && <p className="text-[11px] text-gray-400 mt-0.5">{description}</p>}
      </div>
      <button
        onClick={() => onChange(!checked)}
        className={`relative w-10 h-5.5 rounded-full transition-colors duration-200 ${
          checked ? 'bg-blue-500' : 'bg-gray-300'
        }`}
        style={{ width: 40, height: 22 }}
      >
        <span
          className={`absolute top-0.5 left-0.5 w-[18px] h-[18px] bg-white rounded-full shadow transition-transform duration-200 ${
            checked ? 'translate-x-[18px]' : 'translate-x-0'
          }`}
        />
      </button>
    </div>
  )

  const SliderRow = ({
    label,
    description,
    value,
    min,
    max,
    step,
    unit,
    onChange,
  }: {
    label: string
    description?: string
    value: number
    min: number
    max: number
    step: number
    unit: string
    onChange: (v: number) => void
  }) => (
    <div className="py-3 px-4 rounded-xl bg-white/50 backdrop-blur-sm">
      <div className="flex items-center justify-between mb-1.5">
        <p className="text-sm font-medium text-gray-700">{label}</p>
        <span className="text-xs font-semibold text-blue-500 bg-blue-50 px-2 py-0.5 rounded-full">
          {value.toLocaleString()} {unit}
        </span>
      </div>
      {description && <p className="text-[11px] text-gray-400 mb-2">{description}</p>}
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-blue-500 h-1.5"
      />
      <div className="flex justify-between mt-1">
        <span className="text-[10px] text-gray-300">{min.toLocaleString()}</span>
        <span className="text-[10px] text-gray-300">{max.toLocaleString()}</span>
      </div>
    </div>
  )

  // ── Theme card previews ────────────────────────────────────────────
  const AutomaticPreview = () => (
    <div className="w-full h-full flex items-center justify-center rounded-xl" style={{ background: 'linear-gradient(135deg,#dbeafe 0%,#ede9fe 50%,#fce7f3 100%)' }}>
      <svg viewBox="0 0 40 40" fill="none" className="w-10 h-10" stroke="#94a3b8" strokeWidth="2" strokeLinecap="round">
        <path d="M20 6a14 14 0 1 1-9.9 4.1"/>
        <path d="M20 6l-4 4 4-4 4 4"/>
        <path d="M20 34l4-4-4 4-4-4"/>
      </svg>
    </div>
  )

  // ── Mini window helper ──
  const MiniWindow = ({
    titleBg, titleBorder, bodyBg, sidebarBg, sidebarBorder,
    accentBar, accentCard, accentCardText, accentIcon,
    textPrimary, textSecondary, barBg, radius = '8px',
  }: {
    titleBg: string; titleBorder: string; bodyBg: string; sidebarBg: string; sidebarBorder: string;
    accentBar: string; accentCard: string; accentCardText: string; accentIcon: string;
    textPrimary: string; textSecondary: string; barBg: string; radius?: string;
  }) => (
    <div className="w-full h-full flex items-center justify-center p-1.5">
      <div className="w-full overflow-hidden shadow-md" style={{ borderRadius: radius, border: `1px solid ${titleBorder}`, aspectRatio: '4/3' }}>
        {/* title bar */}
        <div className="flex items-center gap-1 px-2 py-1" style={{ background: titleBg, borderBottom: `1px solid ${titleBorder}` }}>
          <div className="w-1.5 h-1.5 rounded-full bg-red-400" />
          <div className="w-1.5 h-1.5 rounded-full bg-yellow-400" />
          <div className="w-1.5 h-1.5 rounded-full bg-green-400" />
          <div className="flex-1 mx-1.5 h-1 rounded-full" style={{ background: barBg }} />
        </div>
        {/* body */}
        <div className="flex" style={{ background: bodyBg, height: 'calc(100% - 20px)' }}>
          {/* sidebar */}
          <div className="flex flex-col gap-1 pt-1.5 px-1" style={{ width: 22, background: sidebarBg, borderRight: `1px solid ${sidebarBorder}` }}>
            <div className="h-1 rounded-full" style={{ background: accentBar }} />
            <div className="h-1 rounded-full" style={{ background: textSecondary }} />
            <div className="h-1 rounded-full" style={{ background: textSecondary }} />
          </div>
          {/* content */}
          <div className="flex-1 p-1.5 flex flex-col gap-1">
            <div className="flex gap-1">
              <div className="h-1.5 w-7 rounded" style={{ background: textPrimary }} />
              <div className="h-1.5 w-4 rounded" style={{ background: textSecondary }} />
            </div>
            <div className="h-1 rounded-full w-full" style={{ background: textSecondary }} />
            <div className="h-1 rounded-full w-4/5" style={{ background: textSecondary }} />
            <div className="mt-0.5 p-1 flex gap-1 rounded" style={{ background: accentCard, borderRadius: 4 }}>
              <div className="shrink-0 rounded" style={{ width: 10, height: 10, background: accentIcon }} />
              <div className="flex flex-col gap-0.5 flex-1">
                <div className="h-1 rounded-full w-full" style={{ background: accentCardText }} />
                <div className="h-1 rounded-full w-3/4" style={{ background: accentCardText, opacity: 0.6 }} />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )

  // 1. Default — blue-purple-pink gradient glass
  const DefaultPreview = () => (
    <div className="w-full h-full p-1.5 rounded-xl overflow-hidden" style={{ background: 'linear-gradient(135deg,#bfdbfe 0%,#ede9fe 50%,#fce7f3 100%)' }}>
      <MiniWindow
        titleBg="rgba(255,255,255,0.45)" titleBorder="rgba(255,255,255,0.6)"
        bodyBg="rgba(255,255,255,0.15)" sidebarBg="rgba(255,255,255,0.25)" sidebarBorder="rgba(255,255,255,0.4)"
        accentBar="#818cf8" accentCard="rgba(219,234,254,0.7)" accentCardText="#3b82f6"
        accentIcon="#6366f1" textPrimary="#374151" textSecondary="rgba(156,163,175,0.6)" barBg="rgba(255,255,255,0.4)"
        radius="6px"
      />
    </div>
  )

  // 2. Material Light (M3) — warm #FFFBFE surface, #6750A4 primary
  const MaterialLightPreview = () => (
    <MiniWindow
      titleBg="#E6DFF6" titleBorder="#CAC4D0"
      bodyBg="#FFFBFE" sidebarBg="#F7F2FA" sidebarBorder="#E6DFF6"
      accentBar="#6750A4" accentCard="#EADDFF" accentCardText="#21005D"
      accentIcon="#6750A4" textPrimary="#1C1B1F" textSecondary="#CAC4D0" barBg="#E6DFF6"
      radius="10px"
    />
  )

  // 3. Material Dark (M3) — #1C1B1F surface, #D0BCFF primary
  const MaterialDarkPreview = () => (
    <MiniWindow
      titleBg="#2B2930" titleBorder="#49454F"
      bodyBg="#1C1B1F" sidebarBg="#2B2930" sidebarBorder="#49454F"
      accentBar="#D0BCFF" accentCard="#4F378B" accentCardText="#EADDFF"
      accentIcon="#D0BCFF" textPrimary="#E6E1E5" textSecondary="#49454F" barBg="#49454F"
      radius="10px"
    />
  )

  // 4. Monochrome Light — pure gray scale
  const MonoLightPreview = () => (
    <MiniWindow
      titleBg="#F5F5F5" titleBorder="#E0E0E0"
      bodyBg="#FFFFFF" sidebarBg="#FAFAFA" sidebarBorder="#E0E0E0"
      accentBar="#212121" accentCard="#F5F5F5" accentCardText="#212121"
      accentIcon="#616161" textPrimary="#212121" textSecondary="#BDBDBD" barBg="#E0E0E0"
      radius="6px"
    />
  )

  // 5. Monochrome Dark — dark gray scale
  const MonoDarkPreview = () => (
    <MiniWindow
      titleBg="#1E1E1E" titleBorder="#2C2C2C"
      bodyBg="#121212" sidebarBg="#1E1E1E" sidebarBorder="#2C2C2C"
      accentBar="#FFFFFF" accentCard="#2C2C2C" accentCardText="#EEEEEE"
      accentIcon="#9E9E9E" textPrimary="#EEEEEE" textSecondary="#424242" barBg="#2C2C2C"
      radius="6px"
    />
  )

  const themeCards: { id: Theme; label: string; preview: React.ReactNode; cardBg: string; badge?: string }[] = [
    {
      id: 'system',
      label: 'Automatic',
      preview: <AutomaticPreview />,
      cardBg: 'bg-white/50',
    },
    {
      id: 'default',
      label: 'Default',
      preview: <DefaultPreview />,
      cardBg: 'bg-white/50',
      badge: 'Current',
    },
    {
      id: 'material-light',
      label: 'Material Light',
      preview: <MaterialLightPreview />,
      cardBg: 'bg-white/60',
    },
    {
      id: 'material-dark',
      label: 'Material Dark',
      preview: <MaterialDarkPreview />,
      cardBg: 'bg-gray-900/50',
    },
    {
      id: 'mono-light',
      label: 'Mono Light',
      preview: <MonoLightPreview />,
      cardBg: 'bg-white/60',
    },
    {
      id: 'mono-dark',
      label: 'Mono Dark',
      preview: <MonoDarkPreview />,
      cardBg: 'bg-gray-900/50',
    },
  ]

  return (
    <div className="flex-1 overflow-y-auto">
      <div className="max-w-2xl mx-auto px-10 pt-24 pb-16">
        <h1 className="text-4xl font-light text-gray-400 mb-10 text-center">Settings</h1>

        <div className="flex flex-col gap-8">

          {/* ── Appearance ── */}
          <section>
            <SectionTitle icon={<Monitor size={15} />} label="Appearance" />
            <div className="bg-white/40 backdrop-blur-sm rounded-2xl p-5">
              <p className="text-sm font-semibold text-gray-700 mb-0.5">What color theme do you prefer?</p>
              <p className="text-[11px] text-gray-400 mb-5">
                Choose 'Automatic' to automatically change to match your system settings.
              </p>
              <div className="grid grid-cols-3 gap-3">
                {themeCards.map((card) => {
                  const active = theme === card.id
                  const isDarkCard = card.id === 'material-dark' || card.id === 'mono-dark'
                  return (
                    <button
                      key={card.id}
                      onClick={() => setTheme(card.id)}
                      className={`relative flex flex-col items-center rounded-2xl transition-all duration-200 overflow-visible focus:outline-none ${
                        active
                          ? 'ring-2 ring-gray-800 shadow-lg scale-[1.03]'
                          : 'ring-1 ring-white/60 hover:ring-gray-300 hover:scale-[1.01] shadow-sm'
                      } ${card.cardBg} backdrop-blur-sm`}
                    >
                      {/* Checkmark badge */}
                      {active && (
                        <span className="absolute -top-2.5 -right-2.5 z-10 w-6 h-6 rounded-full bg-green-500 flex items-center justify-center shadow-md">
                          <Check size={13} strokeWidth={3} className="text-white" />
                        </span>
                      )}

                      {/* "Current" pill on default theme when not active */}
                      {card.badge && !active && (
                        <span className="absolute -top-2 -right-2 z-10 px-1.5 py-0.5 rounded-full bg-blue-500 text-white text-[9px] font-bold shadow">
                          {card.badge}
                        </span>
                      )}

                      {/* Preview area */}
                      <div className="w-full h-24 overflow-hidden rounded-t-2xl">
                        {card.preview}
                      </div>

                      {/* Label */}
                      <div className={`py-2.5 w-full text-center border-t ${isDarkCard ? 'border-white/10' : 'border-black/5'}`}>
                        <span className={`text-[11px] font-semibold ${active ? (isDarkCard ? 'text-white' : 'text-gray-800') : (isDarkCard ? 'text-gray-400' : 'text-gray-500')}`}>
                          {card.label}
                        </span>
                      </div>
                    </button>
                  )
                })}
              </div>
            </div>
          </section>

          {/* ── Scan Preferences ── */}
          <section>
            <SectionTitle icon={<ScanSearch size={15} />} label="Scan Preferences" />
            <div className="flex flex-col gap-3">
              <SliderRow
                label="Quick Scan — MFT record limit"
                description="Maximum MFT entries read during a Quick Scan."
                value={quickScanLimit}
                min={10}
                max={500}
                step={10}
                unit="k records"
                onChange={setQuickScanLimit}
              />
              <SliderRow
                label="Deep Scan — MFT record limit"
                description="Maximum MFT entries read during a Deep Scan (higher = slower)."
                value={deepScanLimit}
                min={100}
                max={2000}
                step={100}
                unit="k records"
                onChange={setDeepScanLimit}
              />
              <Toggle
                checked={skipSystemFiles}
                onChange={setSkipSystemFiles}
                label="Skip system & hidden files"
                description="Exclude $MFT, pagefile.sys and other OS files from results."
              />
            </div>
          </section>

          {/* ── Recovery ── */}
          <section>
            <SectionTitle icon={<FolderOpen size={15} />} label="Recovery" />
            <div className="flex flex-col gap-3">
              <Toggle
                checked={autoOpenFolder}
                onChange={setAutoOpenFolder}
                label="Open destination folder after recovery"
                description="Automatically open the save location when recovery finishes."
              />
            </div>
          </section>

          {/* ── Notifications ── */}
          <section>
            <SectionTitle
              icon={notifications ? <Bell size={15} /> : <BellOff size={15} />}
              label="Notifications"
            />
            <div className="flex flex-col gap-3">
              <Toggle
                checked={notifications}
                onChange={setNotifications}
                label="Scan completion notifications"
                description="Show a system notification when a scan finishes."
              />
            </div>
          </section>

          {/* ── Danger Zone ── */}
          <section>
            <SectionTitle icon={<Shield size={15} />} label="Danger Zone" />
            <div className="py-3 px-4 rounded-xl bg-red-50/60 backdrop-blur-sm border border-red-100 flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-red-700">Clear scan cache</p>
                <p className="text-[11px] text-red-400 mt-0.5">
                  Removes all cached scan results from disk.
                </p>
              </div>
              <button className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-red-100 hover:bg-red-200 text-red-600 text-xs font-semibold transition-colors">
                <Trash2 size={13} />
                Clear
              </button>
            </div>
          </section>

        </div>
      </div>
    </div>
  )
}

export default Settings
