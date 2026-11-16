# frozen_string_literal: true

require_relative 'harel_machines/version'

module HarelMachines
  # What a transition does: the states it exits (innermost first), the states
  # it enters (outermost first) and the leaves it settles in, each a frozen
  # Array of state names. An answer only: like the core's `Transition`, which
  # only a chart builds, it has no public constructor, so every one that
  # exists came from a Chart (or from reflection: see History#record).
  #
  # It marshals as its three lists. Loading one takes them through the
  # extension's checked copy (exactly three Arrays of distinct Symbols, each
  # copied and frozen), so a loaded Transition is as frozen, all the way
  # down, as one a chart answered; tampered data raises TypeError or
  # RangeError.
  Transition = Data.define(:exit, :entry, :configuration) do
    private

    def marshal_dump = [exit, entry, configuration]

    def marshal_load(fields)
      exit, entry, configuration = self.class.__send__(:__checked_fields, fields)
      initialize(exit:, entry:, configuration:)
    end
  end
end

# Native is required: no algorithm fallback. It resolves Transition above
# once, so it must exist first.
require 'harel_machines_native/harel_machines_native'

module HarelMachines
  # Only a Chart builds a Transition: `new`, `[]` and `with` are gone. The
  # classes are final: subclassing raises TypeError and `allocate` is gone.
  # That stops accidents, not reflection (Ruby binds a named subclass's
  # constant before `inherited` raises; `Marshal.load` and a hand-bound
  # `Data#initialize` skip the constructor), so the extension is the
  # authority: what it reads back must be exactly its class, and is checked
  # again against the chart.
  Transition.singleton_class.send(:private, :new, :[], :__checked_fields)
  Transition.send(:undef_method, :with)
  [Transition, Chart, Chart::Declaration, History].each do |final|
    final.singleton_class.send(:undef_method, :allocate) if final.respond_to?(:allocate)
    final.define_singleton_method(:inherited) { |_| raise TypeError, "#{final} cannot be subclassed" }
  end
  Chart::Declaration.singleton_class.send(:undef_method, :new) if Chart::Declaration.respond_to?(:new)
end
