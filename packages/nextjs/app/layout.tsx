import dynamic from "next/dynamic";
import { AuthProvider } from "./contexts/AuthContext";
import "@rainbow-me/rainbowkit/styles.css";
import { ThemeProvider } from "~~/components/ThemeProvider";
import "~~/styles/globals.css";
import { getMetadata } from "~~/utils/scaffold-eth/getMetadata";

// Dynamic import with SSR disabled to avoid indexedDB errors during build
const ScaffoldEthAppWithProviders = dynamic(
  () => import("~~/components/ScaffoldEthAppWithProviders").then(mod => mod.ScaffoldEthAppWithProviders),
  { ssr: false },
);

export const metadata = getMetadata({
  title: "Trustless Werewolf",
  description: "Trustless social deduction game on Ethereum",
});

const ScaffoldEthApp = ({ children }: { children: React.ReactNode }) => {
  return (
    <html suppressHydrationWarning>
      <body>
        <ThemeProvider enableSystem>
          <AuthProvider>
            <ScaffoldEthAppWithProviders>{children}</ScaffoldEthAppWithProviders>
          </AuthProvider>
        </ThemeProvider>
      </body>
    </html>
  );
};

export default ScaffoldEthApp;
