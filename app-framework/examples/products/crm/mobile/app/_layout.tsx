import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";

export default function RootLayout() {
  return (
    <>
      <Stack
        screenOptions={{
          headerStyle: { backgroundColor: "#f8fafc" },
          headerTintColor: "#123443",
          headerTitleStyle: { fontWeight: "700" }
        }}
      />
      <StatusBar style="auto" />
    </>
  );
}
