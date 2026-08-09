package example;

import java.util.Objects;

interface Named {
    String name();
}

class Greeter implements Named {
    private final String prefix;

    Greeter(String prefix) {
        this.prefix = Objects.requireNonNull(prefix);
    }

    public String name() {
        return prefix;
    }

    String greet(String person) {
        return name() + ", " + person;
    }
}
