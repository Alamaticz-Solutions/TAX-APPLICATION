import * as fs from "fs";
import * as path from "path";
import React from "react";
import { render } from "@testing-library/react-native";

import { pdsNativeTokens } from "../src/design/pdsNativeTokens";
import { createAppfwMobileGraphqlClient, getAppfwMobileEntityClient } from "../src/generated/AppfwMobileDataClient";
import { AppfwEntityScreen } from "../src/generated/AppfwEntityScreen";
import { mobileContract } from "../src/generated/appfw-mobile-contract";

const npmAuditEvidence = require("../.appfw-mobile/npm-audit-evidence.json");
const npmAuditDispositionTemplate = require("../.appfw-mobile/npm-audit-disposition.template.json");
const deviceEvidence = require("../.appfw-mobile/device-evidence.json");
const storeTrackEvidence = require("../.appfw-mobile/store-track-evidence.json");

describe("CRM mobile scaffold contract", () => {
  it("retains generated contract routing for primary account workflows", () => {
    const account = mobileContract.primaryEntities.find((entity) => entity.name === "Account");

    expect(mobileContract.product).toBe("crm");
    expect(mobileContract.nativeRuntime).toBe("react-native-expo-new-architecture");
    expect(account).toMatchObject({
      caption: "Accounts",
      mobileRoute: "entities/accounts",
      readOperations: ["accounts"],
      detailOperation: "account"
    });
  });

  it("emits generated route shells for every primary entity", () => {
    const appRoot = path.resolve(__dirname, "..");
    const generatedScreen = path.join(appRoot, "src/generated/AppfwEntityScreen.tsx");

    expect(fs.existsSync(generatedScreen)).toBe(true);

    for (const entity of mobileContract.primaryEntities) {
      const routeFile = path.join(appRoot, "app", `${entity.mobileRoute}.tsx`);

      expect(fs.existsSync(routeFile)).toBe(true);
      const routeSource = fs.readFileSync(routeFile, "utf8");
      expect(routeSource).toContain(`entityName="${entity.name}"`);
      expect(routeSource).toContain("AppfwEntityScreen");
    }
  });

  it("renders the generated entity screen for a contract entity", () => {
    const entity = mobileContract.primaryEntities[0];
    if (!entity) {
      throw new Error("Expected the generated mobile contract to include a primary entity");
    }

    const screen = render(React.createElement(AppfwEntityScreen, { entityName: entity.name }));

    expect(screen.getByText(entity.caption)).toBeTruthy();
  });

  it("binds generated route shells to generated GraphQL request metadata", async () => {
    const accountClient = getAppfwMobileEntityClient("Account");
    const listRequest = accountClient.buildListRequest({ limit: 10 });
    const recordRequest = accountClient.buildRecordRequest("rec_test");

    expect(accountClient.listOperation.graphqlName).toBe("queryAccounts");
    expect(accountClient.recordOperation.graphqlName).toBe("findAccount");
    expect(accountClient.selectionPreset).toEqual(expect.arrayContaining(["id", "name"]));
    expect(listRequest).toMatchObject({
      entityName: "Account",
      kind: "list",
      operationName: "AppfwMobileAccountList",
      requiresAuth: true,
      requiresTenant: true,
      variables: { limit: 10 }
    });
    expect(listRequest.query).toContain("queryAccounts");
    expect(listRequest.query).toContain("nodes");
    expect(listRequest.query).toContain("pageInfo");
    expect(recordRequest).toMatchObject({
      kind: "record",
      operationName: "AppfwMobileAccountRecord",
      variables: { id: "rec_test" }
    });
    expect(recordRequest.query).toContain("findAccount");

    const fetchImpl = jest.fn(async () => ({
      json: async () => ({
        data: { queryAccounts: { nodes: [], totalCount: 0 } },
        extensions: {
          request_id: "req-123",
          correlation_id: "corr-123"
        }
      })
    })) as unknown as typeof fetch;
    const graphqlClient = createAppfwMobileGraphqlClient({
      endpoint: "https://example.test/graphql",
      getAccessToken: () => "token",
      getTenantId: () => "tenant",
      fetchImpl
    });
    const result = await graphqlClient.execute(listRequest);

    expect(result).toMatchObject({
      state: "loaded",
      requestId: "req-123",
      correlationId: "corr-123"
    });
    expect(fetchImpl).toHaveBeenCalledWith(
      "https://example.test/graphql",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          authorization: "Bearer token",
          "x-tenant-id": "tenant"
        })
      })
    );
  });

  it("keeps governed actions behind intent preview and audit requirements", () => {
    expect(mobileContract.governedActions).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: "AccountHealthReview",
          requiresIntentPreview: true,
          requiresAuditTrail: true
        })
      ])
    );
  });

  it("exposes PDS native tokens required by the scaffold screens", () => {
    expect(pdsNativeTokens.color.action).toBe("#0077a8");
    expect(pdsNativeTokens.color.brandDeep).toBe("#123443");
    expect(pdsNativeTokens.radius.panel).toBeGreaterThan(pdsNativeTokens.radius.control);
    expect(pdsNativeTokens.spacing.screen).toBeGreaterThanOrEqual(pdsNativeTokens.spacing.section);
  });

  it("keeps legacy mobile evidence non-authoritative for candidate and release readiness", () => {
    expect(npmAuditDispositionTemplate).toMatchObject({
      lane: "U5",
      name: "npm-audit-runtime-disposition",
      template_only: true,
      release_approved: false,
      audit_evidence: "mobile/.appfw-mobile/npm-audit-evidence.json",
      verifier_contract: {
        approved_filename: "mobile/.appfw-mobile/npm-audit-disposition.json",
        mobile_test_treats_template_as_release_ready: false
      }
    });
    expect(npmAuditDispositionTemplate.current_evidence_summary).toEqual(
      npmAuditEvidence.audit_summary
    );
    expect(deviceEvidence).toMatchObject({
      status: "not-run",
      static_scaffold_only: true,
      template_only: true,
      candidate_ready: false,
      release_ready: false,
      verifier_contract: {
        authoritative: false,
        mobile_test_treats_placeholder_as_release_ready: false,
        successor: "source-bound-mobile-candidate-checker"
      }
    });
    expect(deviceEvidence.verifier_contract.legacy_diagnostic_requires).toEqual(
      expect.arrayContaining([
        "non-placeholder simulator or physical-device smoke evidence",
        "target platform, OS version, app build identifier, and runtime version"
      ])
    );
    expect(deviceEvidence.replacement_evidence_schema).toMatchObject({
      status: "passed",
      legacy_condition_satisfied: true,
      candidate_ready: false,
      release_ready: false,
      template_only: false
    });
    expect(deviceEvidence.replacement_evidence_schema.required_fields).toEqual(
      expect.arrayContaining([
        "platform",
        "device_or_simulator",
        "app_build_identifier",
        "workflow_matrix",
        "auth_tenant_policy_checks"
      ])
    );
    expect(storeTrackEvidence).toMatchObject({
      status: "not-run",
      static_scaffold_only: true,
      template_only: true,
      candidate_ready: false,
      release_ready: false,
      verifier_contract: {
        authoritative: false,
        mobile_test_treats_placeholder_as_release_ready: false,
        successor: "source-bound-mobile-candidate-checker"
      }
    });
    expect(storeTrackEvidence.verifier_contract.legacy_diagnostic_requires).toEqual(
      expect.arrayContaining([
        "store or enterprise track name and submission/build identifier",
        "signed binary digest, signing identity, runtime version, and update channel"
      ])
    );
    expect(storeTrackEvidence.replacement_evidence_schema).toMatchObject({
      status: "submitted-or-certified",
      legacy_condition_satisfied: true,
      candidate_ready: false,
      release_ready: false,
      template_only: false
    });
    expect(storeTrackEvidence.replacement_evidence_schema.required_fields).toEqual(
      expect.arrayContaining([
        "distribution_channel",
        "track_name",
        "submitted_build_id",
        "signed_binary_digest",
        "runtime_version"
      ])
    );
  });
});
