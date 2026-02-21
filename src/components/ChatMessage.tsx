import { User, Sparkles } from 'lucide-react'
import { ChatMessage as ChatMessageType } from '../types'

interface ChatMessageProps {
  message: ChatMessageType
}

const ChatMessage = ({ message }: ChatMessageProps) => {
  const isUser = message.role === 'user'

  return (
    <div className={`flex gap-4 animate-fade-in ${isUser ? 'flex-row-reverse' : ''}`}>
      {/* Avatar */}
      <div className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 ${
        isUser 
          ? 'bg-gradient-to-br from-slate-700 to-slate-800' 
          : 'bg-gradient-to-br from-blue-400 to-purple-500'
      }`}>
        {isUser ? (
          <User size={16} className="text-slate-300" />
        ) : (
          <Sparkles size={16} className="text-white" />
        )}
      </div>

      {/* Message Content */}
      <div className={`flex-1 ${isUser ? 'flex justify-end' : ''}`}>
        <div
          className={`inline-block px-5 py-3 rounded-2xl max-w-2xl ${
            isUser
              ? 'bg-gradient-to-br from-blue-500/20 to-purple-500/20 border border-blue-500/30 text-slate-100'
              : 'bg-slate-800/50 border border-slate-700/50 text-slate-200'
          }`}
        >
          <p className="text-sm leading-relaxed whitespace-pre-wrap">{message.content}</p>
          <span className="text-xs text-slate-500 mt-2 block">
            {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
          </span>
        </div>
      </div>
    </div>
  )
}

export default ChatMessage
