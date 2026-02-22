import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'

export type Theme =
  | 'system'
  | 'default'
  | 'material-light'
  | 'material-dark'
  | 'mono-light'
  | 'mono-dark'

interface ThemeCtxValue {
  theme: Theme
  setTheme: (t: Theme) => void
}

const ThemeContext = createContext<ThemeCtxValue>({
  theme: 'default',
  setTheme: () => {},
})

/** Resolves 'system' to a concrete theme id, everything else passes through. */
function resolveTheme(t: Theme): string {
  if (t === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'material-dark'
      : 'material-light'
  }
  return t
}

export const ThemeProvider = ({ children }: { children: ReactNode }) => {
  const [theme, setThemeState] = useState<Theme>(
    () => (localStorage.getItem('app-theme') as Theme | null) ?? 'default'
  )

  const setTheme = (t: Theme) => {
    setThemeState(t)
    localStorage.setItem('app-theme', t)
  }

  useEffect(() => {
    const apply = () =>
      document.documentElement.setAttribute('data-theme', resolveTheme(theme))

    apply()

    if (theme === 'system') {
      const mq = window.matchMedia('(prefers-color-scheme: dark)')
      mq.addEventListener('change', apply)
      return () => mq.removeEventListener('change', apply)
    }
  }, [theme])

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  )
}

export const useTheme = () => useContext(ThemeContext)
