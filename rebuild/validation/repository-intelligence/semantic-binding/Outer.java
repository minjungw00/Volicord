// Self-authored CC0-1.0 language control; not historical repository evidence.
public class Outer {
    static int outerHits, anonymousHits, nestedHits, staticHits, deepHits;
    void ping() { outerHits++; }
    void local() { this.ping(); }
    void lexical() { ping(); }
    static void staticPing() { staticHits++; }
    static void qualified() { Outer.staticPing(); }
    void lambda() {
        Runnable task = () -> this.ping();
        task.run();
    }
    class Named {
        void ping() { nestedHits++; }
        void named() { this.ping(); }
        void namedLambda() {
            Runnable task = () -> this.ping();
            task.run();
        }
    }
    Runnable create() {
        return new Runnable() {
            void ping() { anonymousHits++; }
            void anonymous() { this.ping(); }
            void qualifiedInsideAnonymous() { Outer.staticPing(); }
            void anonymousLambda() {
                Runnable task = () -> this.ping();
                task.run();
            }
            class Inside {
                void ping() { nestedHits += 1; }
                void namedInsideAnonymous() { this.ping(); }
            }
            Runnable deep() {
                return new Runnable() {
                    void ping() { deepHits++; }
                    void deepAnonymous() { this.ping(); }
                    public void run() { deepAnonymous(); }
                };
            }
            public void run() {
                anonymous();
                anonymousLambda();
                new Inside().namedInsideAnonymous();
                deep().run();
            }
        };
    }
    static class Holder {
        final Runnable task;
        Holder(Runnable task) { this.task = task; }
    }
    void argumentLambda() {
        Holder holder = new Holder(() -> this.ping()) {
            void ping() { anonymousHits++; }
        };
        holder.task.run();
    }
    public static void main(String[] args) {
        Outer outer = new Outer();
        outer.local();
        outer.lexical();
        outer.lambda();
        outer.argumentLambda();
        Outer.qualified();
        Named named = outer.new Named();
        named.named();
        named.namedLambda();
        outer.create().run();
        if (outerHits != 4 || anonymousHits != 2 || nestedHits != 3
                || staticHits != 1 || deepHits != 1) {
            throw new AssertionError("receiver ownership changed");
        }
        System.out.println("Java receiver ownership controls passed");
    }
}
