import { Suspense, useState } from 'react';
import { Outlet } from 'react-router';
import { AppShell as PdsAppShell } from '@appfw/pds-health-components/layout';
import { Skeleton } from '@appfw/pds-health-components/surfaces';
import { Header } from '../components/layout/Header';
import { SidebarBrand, SidebarFooter, SidebarNav } from '../components/layout/Sidebar';

/**
 * Application chrome: the real PDS `AppShell` (resizable nav rail, a top bar, a scrolling
 * content region). The sidebar uses PDS's own width contract (288 px default, 240-360 px when
 * resized, drawer below 960 px); this product only owns the storage key that remembers the width.
 */
const SIDEBAR_WIDTH_KEY = 'tax-doc-routing.frontend.sidebarWidth';
const DEFAULT_WIDTH = 288;

function readSidebarWidth(): number {
  try {
    const stored = Number(window.localStorage.getItem(SIDEBAR_WIDTH_KEY));
    return Number.isFinite(stored) && stored >= 240 && stored <= 360 ? stored : DEFAULT_WIDTH;
  } catch {
    return DEFAULT_WIDTH;
  }
}

export function AppShell() {
  const [width, setWidth] = useState<number>(() => readSidebarWidth());

  return (
    <PdsAppShell
      brand={<SidebarBrand />}
      navigation={<SidebarNav />}
      footer={<SidebarFooter />}
      topBar={<Header />}
      navigationLabel="Primary navigation"
      viewportBounded
      sidebarResizable
      sidebarWidth={width}
      defaultSidebarWidth={DEFAULT_WIDTH}
      onSidebarWidthChange={setWidth}
      onSidebarWidthCommit={(next) => {
        try {
          window.localStorage.setItem(SIDEBAR_WIDTH_KEY, String(next));
        } catch {
          /* the width just won't be remembered */
        }
      }}
    >
      <Suspense fallback={<Skeleton variant="card" lines={6} label="Loading the page" />}>
        <Outlet />
      </Suspense>
    </PdsAppShell>
  );
}
