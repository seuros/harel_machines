# frozen_string_literal: true

require_relative 'test_helper'

class HistoryTest < Minitest::Test
  include ChartHelpers

  def setup
    @chart = door
    @history = H::History.new(@chart)
  end

  def test_shallow_history_restores_the_child_and_enters_it_at_its_initial
    assert_equal :recorded, @history.record(@chart.resolve(%i[free], :open, :closed, :external))

    step = @chart.resolve_with_history(%i[closed], :closed, :open, :external, @history)

    assert_equal %i[open wide latched], step.entry
    assert_equal %i[latched], step.configuration
  end

  def test_without_history_the_initial_child_is_entered_again
    @history.record(@chart.resolve(%i[free], :open, :closed, :external))

    assert_equal %i[open ajar], @chart.resolve(%i[closed], :closed, :open, :external).entry
  end

  def test_a_history_that_recorded_nothing_falls_back_to_the_initial_child
    step = @chart.resolve_with_history(%i[closed], :closed, :open, :external, @history)

    assert_equal %i[open ajar], step.entry
  end

  def test_entering_from_nothing_reads_history_too
    @history.record(@chart.resolve(%i[free], :open, :closed, :external))

    assert_equal %i[door open wide latched], @chart.enter_with_history(:open, @history).entry
  end

  def test_deep_history_restores_the_whole_configuration
    chart = door(:deep)
    history = H::History.new(chart)
    history.record(chart.resolve(%i[free], :open, :closed, :external))

    step = chart.resolve_with_history(%i[closed], :closed, :open, :external, history)

    assert_equal %i[open wide free], step.entry
    assert_equal %i[free], step.configuration
  end

  def test_deep_history_restores_every_region_of_a_parallel_state
    chart = H::Chart.build do |c|
      c.compound :radio, parent: nil, initial: :off
      c.atomic :off, parent: :radio
      c.parallel :on, parent: :radio, history: :deep
      c.compound :volume, parent: :on, initial: :quiet
      c.atomic :quiet, parent: :volume
      c.atomic :loud, parent: :volume
      c.compound :link, parent: :on, initial: :online
      c.atomic :online, parent: :link
      c.atomic :offline, parent: :link
    end
    history = H::History.new(chart)
    history.record(chart.resolve(%i[loud offline], :on, :off, :external))

    step = chart.resolve_with_history(%i[off], :off, :on, :external, history)

    assert_equal %i[on volume loud link offline], step.entry
    assert_equal %i[loud offline], step.configuration
  end

  def test_what_is_remembered_reads_back_and_restores
    @history.record(@chart.resolve(%i[free], :open, :closed, :external))

    assert_equal [%i[door open], %i[open wide], %i[wide free]], @history.remembered
    assert_predicate @history.remembered, :frozen?
    assert_equal :wide, @history.last_child(:open)
    assert_nil @history.last_child(:closed)
    restored = H::History.restored(@chart, @history.remembered.reverse)
    assert_equal @history.remembered, restored.remembered
  end

  def test_a_history_no_transitions_leave_is_refused
    assert_raises(RangeError) { H::History.restored(@chart, [%i[open wide]]) }
    assert_raises(RangeError) { H::History.restored(@chart, [%i[door ajar]]) }
    assert_raises(RangeError) { H::History.restored(@chart, [%i[nowhere ajar]]) }
    assert_raises(TypeError) { H::History.restored(@chart, [[:open]]) }
    assert_raises(TypeError) { H::History.restored(@chart, [%w[open ajar]]) }
  end

  def test_a_history_of_another_chart_is_refused
    elsewhere = H::History.new(door)

    assert_raises(RangeError) { @chart.resolve_with_history(%i[closed], :closed, :open, :external, elsewhere) }
    assert_raises(RangeError) { @chart.enter_with_history(:open, elsewhere) }
    assert_raises(TypeError) { @chart.enter_with_history(:open, @chart) }
  end

  # Specified by hand: an explicitly targeted state is entered as named, so
  # the deep history of an ancestor restores nothing below it.
  def test_an_ancestor_s_history_does_not_reach_into_an_explicit_target
    chart = H::Chart.build do |c|
      c.compound :root, parent: nil, initial: :off, history: :deep
      c.atomic :off, parent: :root
      c.compound :work, parent: :root, initial: :first
      c.atomic :first, parent: :work
      c.atomic :last, parent: :work
    end
    history = H::History.new(chart)
    leave = chart.resolve(%i[last], :work, :off, :external)
    history.record(leave)

    assert_equal %i[work first], chart.resolve_with_history(%i[off], :off, :work, :external, history).entry
    assert_equal %i[root work first], chart.enter_with_history(:work, history).entry
    assert_equal %i[root work last], chart.enter_with_history(:root, history).entry
  end
end
