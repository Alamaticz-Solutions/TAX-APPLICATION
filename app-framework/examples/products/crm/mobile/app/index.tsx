import { Link } from "expo-router";
import { ScrollView, Text, View } from "react-native";

import { mobileContract } from "../src/generated/appfw-mobile-contract";
import { pdsNativeTokens } from "../src/design/pdsNativeTokens";

export default function HomeScreen() {
  return (
    <ScrollView
      style={{ flex: 1, backgroundColor: pdsNativeTokens.color.canvas }}
      contentContainerStyle={{ padding: 20, gap: 16 }}
    >
      <View style={{ gap: 8 }}>
        <Text style={{ color: pdsNativeTokens.color.brandDeep, fontSize: 13, fontWeight: "800", letterSpacing: 1.2 }}>
          PDS CRM
        </Text>
        <Text style={{ color: pdsNativeTokens.color.textStrong, fontSize: 30, fontWeight: "800" }}>
          Mobile workspace
        </Text>
        <Text style={{ color: pdsNativeTokens.color.textMuted, fontSize: 16, lineHeight: 23 }}>
          Native CRM shell for account review, activity follow-up, and governed actions.
        </Text>
      </View>

      <View
        style={{
          backgroundColor: pdsNativeTokens.color.surface,
          borderColor: pdsNativeTokens.color.border,
          borderRadius: 18,
          borderWidth: 1,
          padding: 18,
          gap: 10
        }}
      >
        <Text style={{ color: pdsNativeTokens.color.textStrong, fontSize: 18, fontWeight: "700" }}>
          Contract source
        </Text>
        <Text style={{ color: pdsNativeTokens.color.textMuted, lineHeight: 21 }}>
          {mobileContract.productDisplayName} consumes {mobileContract.generatedContractSource} and keeps mobile source
          product-owned until the mobile generator target exists.
        </Text>
        <Link href="/activity-queue" style={{ color: pdsNativeTokens.color.action, fontSize: 16, fontWeight: "700" }}>
          Open activity queue
        </Link>
        <Link href="/entities/accounts" style={{ color: pdsNativeTokens.color.action, fontSize: 16, fontWeight: "700" }}>
          Open accounts
        </Link>
      </View>
    </ScrollView>
  );
}
