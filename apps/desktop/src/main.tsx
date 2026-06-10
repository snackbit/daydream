import React from 'react';
import ReactDOM from 'react-dom/client';
import { RouterProvider } from '@tanstack/react-router';
import { TooltipProvider } from '@/components/ui/tooltip';
import { AppStartupGate } from '@/features/startup/components/app-startup-gate';
import './index.css';
import { router } from './routes';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <TooltipProvider>
      <AppStartupGate>
        <RouterProvider router={router} />
      </AppStartupGate>
    </TooltipProvider>
  </React.StrictMode>,
);
