import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

public class PrimitiveTest {

    @Test
    public void addInt() {
        assertEquals(0, Primitive.addInt(0, 0));
        assertEquals(11, Primitive.addInt(1, 10));
    }

    @Test
    public void compareBool() {
        assertEquals(true, Primitive.compareBool(true, true));
        assertEquals(true, Primitive.compareBool(false, false));
        assertEquals(false, Primitive.compareBool(true, false));
        assertEquals(false, Primitive.compareBool(false, true));
    }

}