import { Link, useLocalSearchParams } from "expo-router";
import { useState } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import { pdsNativeTokens } from "../../design/pdsNativeTokens";
import { activityQueueFixture } from "./activityQueueFixture";

export function ActivityContinuationScreen() {
  const { item: itemParam } = useLocalSearchParams<{ item?: string }>();
  const item = activityQueueFixture.find((candidate) => candidate.id === itemParam) ?? activityQueueFixture[0];
  const unknown = Boolean(itemParam && !activityQueueFixture.some((candidate) => candidate.id === itemParam));
  const [preview, setPreview] = useState<"idle" | "reviewing" | "receipt">("idle");
  if (unknown) return <UnknownActivity />;
  return (
    <ScrollView accessibilityLabel="Activity continuation" contentContainerStyle={styles.screen}>
      <Link href="/" style={styles.back}>Back to workspace</Link>
      <Text style={styles.eyebrow}>Activity continuation</Text>
      <Text style={styles.title}>{item.subject}</Text>
      <Text style={styles.account}>{item.account} · {item.owner}</Text>
      <View style={styles.band}>
        <Text style={styles.label}>Due</Text><Text style={styles.value}>{item.dueLabel}</Text>
        <Text style={styles.label}>Status</Text><Text style={styles.value}>{item.status}</Text>
        <Text style={styles.label}>Freshness</Text><Text style={[styles.value, item.freshness === "Stale" && styles.warning]}>{item.freshness}</Text>
      </View>
      <View style={styles.band}><Text style={styles.section}>Read</Text><Text style={styles.body}>{item.summary}</Text><Text style={styles.body}>{item.dependency}</Text></View>
      <View style={styles.preview}>
        <Text style={styles.section}>Local preview</Text>
        <Text style={styles.body}>No action is sent. Review the next consequence locally before continuing.</Text>
        {preview === "idle" && <Pressable accessibilityLabel="Continue local preview" onPress={() => setPreview("reviewing")} style={styles.button}><Text style={styles.buttonText}>Continue</Text></Pressable>}
        {preview === "reviewing" && <View style={styles.actions}><Text accessibilityLabel="Preview ready" style={styles.notice}>Preview ready for review.</Text><Pressable accessibilityLabel="Record local receipt" onPress={() => setPreview("receipt")} style={styles.button}><Text style={styles.buttonText}>Record receipt</Text></Pressable><Pressable accessibilityLabel="Cancel preview" onPress={() => setPreview("idle")} style={styles.cancel}><Text>Cancel</Text></Pressable></View>}
        {preview === "receipt" && <View><Text accessibilityLabel="Local receipt recorded" style={styles.notice}>Local receipt recorded.</Text><Pressable accessibilityLabel="Resume preview" onPress={() => setPreview("reviewing")} style={styles.cancel}><Text>Resume</Text></Pressable></View>}
      </View>
    </ScrollView>
  );
}
function UnknownActivity() { return <View style={styles.screen}><Text style={styles.title}>Activity not found</Text><Text style={styles.body}>This local fixture item is unavailable.</Text><Link accessibilityLabel="Return to workspace" href="/" style={styles.back}>Return to workspace</Link></View>; }
const styles = StyleSheet.create({ screen: { flexGrow: 1, gap: 14, padding: 20, backgroundColor: pdsNativeTokens.color.canvas }, back: { color: pdsNativeTokens.color.action, fontSize: 15, fontWeight: "700" }, eyebrow: { color: "#0077a8", fontSize: 12, fontWeight: "800", letterSpacing: 0, textTransform: "uppercase" }, title: { color: pdsNativeTokens.color.textStrong, fontSize: 28, fontWeight: "800" }, account: { color: pdsNativeTokens.color.textMuted, fontSize: 15 }, band: { gap: 7, padding: 16, borderWidth: 1, borderColor: pdsNativeTokens.color.border, backgroundColor: pdsNativeTokens.color.surface }, label: { color: "#5e6b75", fontSize: 12 }, value: { color: "#1f2a33", fontSize: 16, fontWeight: "700" }, warning: { color: pdsNativeTokens.color.brandDeep }, section: { color: "#1f2a33", fontSize: 18, fontWeight: "800" }, body: { color: "#5e6b75", fontSize: 15, lineHeight: 22 }, preview: { gap: 10, padding: 16, borderLeftWidth: 4, borderLeftColor: pdsNativeTokens.color.action, backgroundColor: pdsNativeTokens.color.canvas }, actions: { gap: 10 }, button: { alignItems: "center", padding: 12, backgroundColor: pdsNativeTokens.color.brandDeep }, buttonText: { color: pdsNativeTokens.color.surface, fontWeight: "800" }, cancel: { alignItems: "center", padding: 10 }, notice: { color: "#123443", fontWeight: "700" } });
