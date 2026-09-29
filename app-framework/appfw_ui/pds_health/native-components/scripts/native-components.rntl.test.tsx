import { fireEvent, render } from "@testing-library/react-native";
import React from "react";
import { AccessibilityInfo, StyleSheet } from "react-native";

import fixture from "../../ix-presentation-contract/fixtures/working-brief.presentation.json";
import {
  EvidenceDisclosure,
  PdsIxPresentation,
  type PdsIxPresentationProps
} from "../src/index";

const presentation = fixture as unknown as PdsIxPresentationProps["presentation"];

test("disclosure exposes expanded state, source callback, Dynamic Type, and a 44-point target", () => {
  const onSourcePress = jest.fn();
  const model = fixture.context.evidence;
  const screen = render(<EvidenceDisclosure model={model} onSourcePress={onSourcePress} />);
  const disclosure = screen.getByRole("button", { name: model.summary });
  expect(disclosure.props.accessibilityState).toEqual({ expanded: false });
  expect(StyleSheet.flatten(disclosure.props.style).minHeight).toBeGreaterThanOrEqual(44);
  fireEvent.press(disclosure);
  expect(screen.getByRole("button", { name: model.summary }).props.accessibilityState).toEqual({ expanded: true });
  const source = screen.getByRole("link");
  expect(StyleSheet.flatten(source.props.style).minHeight).toBeGreaterThanOrEqual(44);
  fireEvent.press(source);
  expect(onSourcePress).toHaveBeenCalledWith("fixture-source-a");
  const scaledText = screen.getAllByText("Sanitized fixture")[0];
  expect(scaledText.props.allowFontScaling).toBe(true);
  expect(scaledText.props.maxFontSizeMultiplier).toBeUndefined();
});

test("composition renders gaps and visible region status, then routes optional callbacks", () => {
  const onContextAction = jest.fn();
  const onStatusAction = jest.fn();
  const onRegionAction = jest.fn();
  const onRegionEdit = jest.fn();
  const withActions = {
    ...fixture,
    context: { ...fixture.context, actionLabel: "Inspect context" },
    workStatus: { ...fixture.workStatus, actionLabel: "Inspect status" },
    response: {
      ...fixture.response,
      regions: [{ ...fixture.response.regions[0], actionLabel: "Challenge finding" }]
    }
  };
  const screen = render(
    <PdsIxPresentation
      onContextAction={onContextAction}
      onStatusAction={onStatusAction}
      onRegionAction={onRegionAction}
      onRegionEdit={onRegionEdit}
      presentation={withActions}
    />
  );
  expect(screen.getByText("Context gaps")).toBeTruthy();
  expect(screen.getByText(/Approval owner is not yet resolved\./)).toBeTruthy();
  expect(screen.getByText("ready")).toBeTruthy();
  fireEvent.press(screen.getByRole("button", { name: "Inspect context" }));
  fireEvent.press(screen.getByRole("button", { name: "Inspect status" }));
  fireEvent.press(screen.getByRole("button", { name: "Challenge finding" }));
  fireEvent.press(screen.getByRole("button", { name: "Edit this section" }));
  expect(onContextAction).toHaveBeenCalledTimes(1);
  expect(onStatusAction).toHaveBeenCalledTimes(1);
  expect(onRegionAction).toHaveBeenCalledWith(expect.objectContaining({ id: "finding", status: "ready" }));
  expect(onRegionEdit).toHaveBeenCalledWith(expect.objectContaining({ id: "finding", status: "ready" }));
});

test("malformed transport renders deterministic non-action fallback and never throws", () => {
  const screen = render(<PdsIxPresentation presentation={{ unexpected: true }} />);
  expect(screen.getByTestId("pds-presentation-invalid")).toBeTruthy();
  expect(screen.getByRole("alert")).toHaveTextContent("This intelligence presentation is unavailable.");
  expect(screen.queryAllByRole("button")).toHaveLength(0);
});

test("top-level announcement fires once per identity revision and not for unchanged rerenders", () => {
  const announce = jest.mocked(AccessibilityInfo.announceForAccessibility);
  announce.mockClear();
  const screen = render(<PdsIxPresentation presentation={presentation} />);
  expect(announce).toHaveBeenLastCalledWith(fixture.announcement);
  expect(announce).toHaveBeenCalledTimes(1);
  screen.rerender(<PdsIxPresentation presentation={{ ...fixture }} />);
  expect(announce).toHaveBeenCalledTimes(1);
  const revision2 = {
    ...fixture,
    identity: { ...fixture.identity, revision: 2 },
    announcement: "Working brief revision 2 is ready."
  };
  screen.rerender(<PdsIxPresentation presentation={revision2} />);
  expect(announce).toHaveBeenLastCalledWith("Working brief revision 2 is ready.");
  expect(announce).toHaveBeenCalledTimes(2);
});
