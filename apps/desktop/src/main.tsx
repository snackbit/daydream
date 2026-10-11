import React from 'react';
import ReactDOM from 'react-dom/client';
import { RouterProvider } from '@tanstack/react-router';
import { TooltipProvider } from '@/components/ui/tooltip';
import { AppStartupGate } from '@/features/startup/components/app-startup-gate';
import { installFrontendFailureLogging } from '@/lib/diagnostics/failure-logging';
import './index.css';
import { router } from './routes';

const removeFailureLogging = installFrontendFailureLogging();
import.meta.hot?.dispose(removeFailureLogging);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <TooltipProvider>
      <AppStartupGate>
        <RouterProvider router={router} />
      </AppStartupGate>
    </TooltipProvider>
  </React.StrictMode>,
);
