"use client";

import { useState, createContext, useContext } from 'react';

interface SidebarContextType {
  collapsed: boolean;
  toggleSidebar: () => void;
  setCollapsed: (collapsed: boolean) => void;
}

const SidebarContext = createContext<SidebarContextType | undefined>(undefined);

export function useSidebarState() {
  const context = useContext(SidebarContext);
  
  if (!context) {
    // Fallback hook for when not wrapped in provider
    const [collapsed, setCollapsedState] = useState(() => {
      if (typeof window !== 'undefined') {
        const savedCollapsed = localStorage.getItem('sidebar-collapsed');
        if (savedCollapsed !== null) {
          return JSON.parse(savedCollapsed);
        }
      }
      return false;
    });

    const setCollapsed = (newCollapsed: boolean) => {
      setCollapsedState(newCollapsed);
      if (typeof window !== 'undefined') {
        localStorage.setItem('sidebar-collapsed', JSON.stringify(newCollapsed));
      }
      // Dispatch custom event to notify sidebar component
      window.dispatchEvent(new CustomEvent('sidebar-toggle', { 
        detail: { collapsed: newCollapsed } 
      }));
    };

    const toggleSidebar = () => {
      setCollapsed(!collapsed);
    };

    return {
      collapsed,
      setCollapsed,
      toggleSidebar,
    };
  }
  
  return context;
}

export { SidebarContext };
