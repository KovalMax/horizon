import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:horizon_comparison/main.dart';

void main() {
  testWidgets('a member can create and complete a task', (tester) async {
    await tester.pumpWidget(const HorizonApp());
    await tester.tap(find.text('Our household').first);
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('+ Add task'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('+ Add task'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Buy tea');
    await tester.tap(find.text('Add task').last);
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('Buy tea'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Buy tea'));
    await tester.pumpAndSettle();
    final tile = tester.widget<CheckboxListTile>(find.ancestor(of: find.text('Buy tea'), matching: find.byType(CheckboxListTile)));
    expect(tile.value, isTrue);
  });

  testWidgets('a member can open and close People', (tester) async {
    await tester.pumpWidget(const HorizonApp());
    await tester.tap(find.text('Our household').first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('People · 2'));
    await tester.pumpAndSettle();
    expect(find.text('Jamie · Editor'), findsOneWidget);
    await tester.tap(find.text('Close'));
    await tester.pumpAndSettle();
    expect(find.text('Jamie · Editor'), findsNothing);
  });

  testWidgets('a member can expand and collapse the calendar', (tester) async {
    await tester.pumpWidget(const HorizonApp());
    await tester.tap(find.text('Our household').first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Expand calendar ↗'));
    await tester.pumpAndSettle();
    expect(find.text('June 2026'), findsOneWidget);
    await tester.tap(find.text('Collapse ↑'));
    await tester.pumpAndSettle();
    expect(find.text('June 2026'), findsNothing);
  });
}
