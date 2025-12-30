'use client';

import { GRAPH_TOUR_STEPS, TourProvider, TourTrigger } from '@/components/onboarding';

interface GraphTourWrapperProps {
  children: React.ReactNode;
}

/**
 * Wraps the graph page with tour functionality.
 * Provides onboarding tour for first-time users.
 */
export function GraphTourWrapper({ children }: GraphTourWrapperProps) {
  return (
    <TourProvider
      steps={GRAPH_TOUR_STEPS}
      storageKey="edgequake.tour.graph.completed"
      autoStart={false} // Don't auto-start, let users trigger it
      onComplete={() => {
        console.log('Graph tour completed');
      }}
    >
      {children}
    </TourProvider>
  );
}

/**
 * Button to start the graph tour.
 * Can be placed in the toolbar or help menu.
 */
export function GraphTourTrigger() {
  return <TourTrigger />;
}

export default GraphTourWrapper;
