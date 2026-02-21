# Natural AI Desktop - Electron + React + TypeScript + Tailwind CSS

A beautiful AI assistant desktop application built with Electron, React, TypeScript, and Tailwind CSS featuring a modern dashboard interface with action cards.

## Features

- 🎨 Beautiful light theme with gradient background (blue to purple/pink)
- 🎯 Dashboard with action card grid for quick tasks
- 💬 Status notifications in the top bar
- 🔔 Shopping cart and notification badges
- 🎤 Voice input support (Press and hold S to speak)
- 📜 Queries history sidebar
- 🪟 Frameless custom window
- ⚡ Fast and responsive
- 📱 Resizable layout

## UI Design

The interface features:
- **Top Bar**: Order status, shopping cart with badge, notifications, and user profile
- **Main Dashboard**: Centered action cards for various tasks:
  - Order food delivery
  - Buy something
  - Order groceries
  - Book a hotel
  - Book a movie
  - Book a flight
  - Get a ride
  - My orders (with active badge)
- **Left Sidebar**: Decorative orbs and queries history dropdown
- **Voice Input**: Keyboard shortcut hint for voice commands

## Tech Stack

- **Electron** - Desktop application framework
- **React 18** - UI library
- **TypeScript** - Type safety
- **Tailwind CSS** - Utility-first CSS framework
- **Vite** - Fast build tool
- **Lucide React** - Beautiful icon set

## Installation

First, install the dependencies:

```bash
npm install
```

## Development

Run the application in development mode:

```bash
npm run dev
```

This will:
1. Start the Vite dev server for React
2. Compile the Electron TypeScript files
3. Launch the Electron application

The app will hot-reload when you make changes to the React code.

## Building

Build the application for production:

```bash
npm run build
```

This will compile both the React app and Electron files into the `dist` directory.

## Running Production Build

After building, you can run the production version:

```bash
npm start
```

## Project Structure

```
natural-ai-desktop/
├── electron/           # Electron main process files
│   ├── main.ts        # Main process entry point
│   └── preload.ts     # Preload script
├── src/               # React application
│   ├── components/    # React components
│   │   ├── TitleBar.tsx
│   │   ├── Sidebar.tsx
│   │   ├── ChatArea.tsx
│   │   └── ChatMessage.tsx
│   ├── App.tsx        # Main App component
│   ├── main.tsx       # React entry point
│   ├── index.css      # Global styles
│   └── types.ts       # TypeScript types
├── index.html         # HTML template
├── package.json       # Dependencies and scripts
├── tsconfig.json      # TypeScript config for React
├── tsconfig.electron.json  # TypeScript config for Electron
├── tailwind.config.js # Tailwind CSS configuration
├── postcss.config.js  # PostCSS configuration
└── vite.config.ts     # Vite configuration
```

## Customization

### Colors

You can customize the color scheme in `tailwind.config.js`:

```javascript
theme: {
  extend: {
    colors: {
      // Add your custom colors here
    },
  },
}
```

### Window Settings

Modify the Electron window settings in `electron/main.ts`:

```typescript
mainWindow = new BrowserWindow({
  width: 1200,
  height: 800,
  // Customize other window options
});
```

## Features to Implement

This is a UI demo. To make it functional, you can:

1. **Connect to an AI API**: Integrate with OpenAI, Anthropic, or other AI services
2. **Persistent Storage**: Add database support for conversation history
3. **Settings Panel**: Add user preferences and configuration options
4. **File Uploads**: Allow users to upload and analyze files
5. **Export Conversations**: Enable exporting chats to markdown or PDF
6. **Keyboard Shortcuts**: Add hotkeys for common actions
7. **Multi-language Support**: Add i18n for multiple languages

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
