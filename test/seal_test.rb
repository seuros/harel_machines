# frozen_string_literal: true

require_relative 'test_helper'

# The classes are final and Transition has no public constructor. Sealing
# stops accidents, not reflection, so the extension is the authority: the one
# value it reads back, a Transition given to History#record, must be exactly
# a Transition, its fields are read from the struct, and its exit set is
# checked against the chart.
class SealTest < Minitest::Test
  include ChartHelpers

  def setup
    @chart = door
    @history = H::History.new(@chart)
    @leaving = @chart.resolve(%i[free], :open, :closed, :external)
  end

  def forge(exit)
    H::Transition.send(:new, exit: exit, entry: [], configuration: [])
  end

  def test_a_transition_cannot_be_built_by_hand
    assert_raises(NoMethodError) { H::Transition.new(exit: [], entry: [], configuration: []) }
    assert_raises(NoMethodError) { H::Transition[[], [], []] }
    assert_raises(NoMethodError) { @leaving.with(exit: []) }
  end

  def test_no_class_can_be_subclassed_or_allocated
    [H::Transition, H::Chart, H::Chart::Declaration, H::History].each do |final|
      assert_raises(TypeError, final.name) { Class.new(final) }
      assert_raises(NoMethodError, final.name) { final.allocate }
    end
    assert_raises(TypeError) { H::Chart.new }
    assert_raises(NoMethodError) { H::Chart::Declaration.new }
  end

  def test_a_marshalled_transition_round_trips_and_records_like_the_original
    copy = Marshal.load(Marshal.dump(@leaving))

    assert_instance_of H::Transition, copy
    assert_equal @leaving, copy
    assert_predicate copy, :frozen?
    [copy.exit, copy.entry, copy.configuration].each do |list|
      assert_predicate list, :frozen?
      assert_raises(FrozenError) { list << :x }
    end
    assert_equal :recorded, @history.record(copy)
    assert_equal [%i[door open], %i[open wide], %i[wide free]], @history.remembered
  end

  # Marshal data written by hand: a Transition dumps as `U`, its class and
  # its three lists. Dumping the class name and the lists as one pair keeps
  # their symbol references in one table; the pair's header becomes `U`.
  def marshalled(fields)
    "\x04\bU".b + Marshal.dump([:'HarelMachines::Transition', fields]).delete_prefix("\x04\b[\a".b)
  end

  # Tampered data is checked on the way in: a String where a Symbol belongs,
  # a state listed twice, a list that is not an Array, the wrong number of
  # lists.
  def test_tampered_marshal_data_is_rejected
    genuine = @chart.enter(:door)
    assert_equal genuine, Marshal.load(marshalled([genuine.exit, genuine.entry, genuine.configuration]))

    assert_raises(TypeError) { Marshal.load(marshalled([[], ['door'], []])) }
    assert_raises(RangeError) { Marshal.load(marshalled([[], %i[door door], []])) }
    assert_raises(TypeError) { Marshal.load(marshalled([[], :door, []])) }
    assert_raises(TypeError) { Marshal.load(marshalled([[], []])) }
    assert_raises(TypeError) { Marshal.load(marshalled({ exit: [] })) }
  end

  def test_native_objects_are_not_marshalled_or_copied
    [@chart, @history].each do |native|
      assert_raises(TypeError) { Marshal.dump(native) }
      assert_raises(TypeError) { native.dup }
      assert_raises(TypeError) { native.clone }
    end
  end

  def test_a_forged_transition_is_checked_against_the_chart
    assert_raises(RangeError) { @history.record(forge(%i[nowhere])) }
    assert_raises(TypeError) { @history.record(forge(%w[wide])) }
    assert_raises(TypeError) { @history.record(forge(Class.new(Array).new(%i[wide]))) }
    # Wide exited without the leaf it held: no transition of the door does that.
    assert_equal :invalid_history, @history.record(forge(%i[wide]))
    # Two children of one compound state were never active together.
    assert_equal :invalid_history, @history.record(forge(%i[latched free]))
    # The right states in the wrong order.
    assert_equal :invalid_history, @history.record(forge(%i[open wide free]))
    assert_empty @history.remembered
  end

  def test_only_exactly_a_transition_is_recorded
    leaked = Class.new(Data.define(:exit, :entry, :configuration)).new(exit: %i[free], entry: [], configuration: [])

    assert_raises(TypeError) { @history.record(leaked) }
    assert_raises(TypeError) { @history.record(@leaving.to_h) }
    assert_raises(TypeError) { @history.record(nil) }
  end

  def test_a_redefined_reader_is_ignored_and_an_opened_singleton_class_is_still_a_transition
    @leaving.singleton_class
    H::Transition.alias_method(:original_exit, :exit)
    H::Transition.define_method(:exit) { raise 'reader ran' }

    assert_equal :recorded, @history.record(@leaving)
  ensure
    H::Transition.alias_method(:exit, :original_exit)
    H::Transition.remove_method(:original_exit)
  end

  def test_a_frozen_history_raises_before_reading_its_argument
    @history.freeze

    assert_raises(FrozenError) { @history.record(:not_a_transition) }
    assert_raises(FrozenError) { H::Chart.build { |c| c.freeze.atomic(:root, parent: nil) } }
  end

  def test_a_declaration_kept_past_its_build_changes_no_chart
    kept = nil
    chart = H::Chart.build do |c|
      kept = c
      c.atomic :root, parent: nil
    end
    kept.atomic :late, parent: :root

    assert_equal %i[root], chart.states
  end
end
