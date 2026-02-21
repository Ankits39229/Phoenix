import { ChevronDown } from 'lucide-react'

const Sidebar = () => {
  return (
    <div className="w-56 flex flex-col justify-between py-12 px-6 relative z-0">
      {/* Top orb */}
      <div className="flex flex-col items-center gap-6">
        <div className="w-20 h-20 rounded-full bg-white/40 backdrop-blur-sm shadow-md"></div>
      </div>
      
      {/* Bottom section */}
      <div className="flex flex-col items-center">
        <div className="w-32 h-32 rounded-full bg-white/40 backdrop-blur-sm shadow-md mb-8"></div>
        <button className="flex items-center gap-2 text-gray-400 hover:text-gray-500 transition-colors text-sm">
          <span>Queries history</span>
          <ChevronDown size={16} />
        </button>
      </div>
    </div>
  )
}

export default Sidebar
