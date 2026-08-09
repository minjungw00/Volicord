package example;

import org.junit.jupiter.api.Test;

class GreeterTest {
    @Test
    void greetsPerson() {
        new Greeter("hello").greet("Ada");
    }
}
