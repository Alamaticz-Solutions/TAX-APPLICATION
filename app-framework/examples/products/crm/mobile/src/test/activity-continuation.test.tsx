import { fireEvent, render, screen } from "@testing-library/react-native";
import { ActivityContinuationScreen } from "../features/activities/ActivityContinuationScreen";

jest.mock("expo-router", () => ({ Link: ({ children, ...props }: any) => children, useLocalSearchParams: () => ({ item: "activity-acme-discovery" }) }));

test("preserves a local preview across interruption and resumes", () => {
  render(<ActivityContinuationScreen />);
  fireEvent.press(screen.getByLabelText("Continue local preview"));
  expect(screen.getByLabelText("Cancel preview")).toBeTruthy();
  fireEvent.press(screen.getByLabelText("Cancel preview"));
  fireEvent.press(screen.getByLabelText("Continue local preview"));
  expect(screen.getByLabelText("Record local receipt")).toBeTruthy();
});

test("renders the deep-linked activity detail with accessible labels", () => {
  render(<ActivityContinuationScreen />);
  expect(screen.getByText("Discovery call follow-up")).toBeTruthy();
  expect(screen.getByLabelText("Activity continuation")).toBeTruthy();
});
